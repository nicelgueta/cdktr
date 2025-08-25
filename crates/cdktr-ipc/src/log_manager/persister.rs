use std::{env, time::Duration};

use crate::log_manager::model::LogMessage;
use cdktr_core::{
    exceptions::{cdktr_result, GenericError},
    get_cdktr_setting,
    utils::data_structures::AsyncQueue,
    zmq_helpers::{get_server_tcp_uri, get_zmq_sub},
};
use cdktr_db::DBClient;
use log::{info, warn};
use tokio::time::{sleep_until, Instant};
use zeromq::SocketRecv;

// write logs to the database every 30 seconds
static CACHE_PERSISTENCE_INTERVAL_MS: u64 = 30_000;

pub async fn start_listener(mut logs_queue: AsyncQueue<LogMessage>) -> Result<(), GenericError> {
    let mut logs_sub_socket = get_zmq_sub(
        &get_server_tcp_uri(
            get_cdktr_setting!(CDKTR_PRINCIPAL_HOST).as_str(),
            get_cdktr_setting!(CDKTR_LOGS_PUBLISHING_PORT, usize),
        ),
        "",
    )
    .await?;
    loop {
        let log_msg = cdktr_result(logs_sub_socket.recv().await)?;
        logs_queue.put(LogMessage::try_from(log_msg)?).await
    }
}

pub async fn start_persistence_loop(db_client: DBClient, mut logs_queue: AsyncQueue<LogMessage>) {
    loop {
        sleep_until(Instant::now() + Duration::from_millis(CACHE_PERSISTENCE_INTERVAL_MS)).await;
        let logs_to_persist = logs_queue.dump().await;
        match persist_cache(&db_client, logs_to_persist).await {
            Ok(()) => (),
            Err(failed_batch) => {
                logs_queue.put_front_multiple(failed_batch);
                warn!("Failed to persist logs to db - will retry on next interval")
                //
            }
        }
    }
}

pub async fn persist_cache(
    db_client: &DBClient,
    logs_to_persist: Vec<LogMessage>,
) -> Result<(), Vec<LogMessage>> {
    info!("Saving {} logs to db", logs_to_persist.len());
    db_client.batch_load("logstore", logs_to_persist).await
}

#[cfg(test)]
mod tests {

    use super::*;
    use cdktr_db::DBClient;

    #[tokio::test]
    async fn test_persist_cache() {
        let db_client = DBClient::new(None).unwrap();
        let mut q = AsyncQueue::new();

        let msg1 = LogMessage::new(
            "test_workflow_id".to_string(),
            "test_workflow_name".to_string(),
            "test_workflow_instance_id".to_string(),
            1234567890 as u64,
            "INFO".to_string(),
            "a log message!".to_string(),
        );
        let msg2 = LogMessage::new(
            "test_workflow_id".to_string(),
            "test_workflow_name".to_string(),
            "test_workflow_instance_id".to_string(),
            234567890 as u64,
            "INFO".to_string(),
            "a second log message!".to_string(),
        );
        q.put(msg1).await;
        q.put(msg2).await;

        persist_cache(&db_client, q.dump().await)
            .await
            .expect("Failed to persist the cached log messages");
        let locked_client = (&db_client).lock_inner_client().await;
        let mut stmt = locked_client.prepare("SELECT * FROM logstore").unwrap();
        let message_iter = stmt
            .query_map([], |row| {
                Ok(LogMessage {
                    workflow_id: row.get(0).unwrap(),
                    workflow_name: row.get(1).unwrap(),
                    workflow_instance_id: row.get(2).unwrap(),
                    timestamp_ms: row.get(3).unwrap(),
                    level: row.get(4).unwrap(),
                    payload: row.get(5).unwrap(),
                })
            })
            .unwrap();
        for (i, msg) in message_iter.enumerate() {
            assert!(msg.is_ok());
            assert!(i < 2)
        }
        drop(locked_client);
    }
}
