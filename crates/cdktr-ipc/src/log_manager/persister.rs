use std::{
    env,
    {collections::VecDeque, time::Duration},
};

use crate::log_manager::model::LogMessage;
use cdktr_core::{
    exceptions::{cdktr_result, GenericError},
    get_cdktr_setting,
    utils::data_structures::AsyncQueue,
    zmq_helpers::{get_server_tcp_uri, get_zmq_sub},
};
use cdktr_db::{get_db_client, get_test_db_client};
use duckdb::{params, Result as DuckResult};
use log::warn;
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

pub async fn start_persistence_loop(mut logs_queue: AsyncQueue<LogMessage>) {
    loop {
        sleep_until(Instant::now() + Duration::from_millis(CACHE_PERSISTENCE_INTERVAL_MS)).await;
        match persist_cache(logs_queue.dump().await, false) {
            Ok(()) => (),
            Err(e) => {
                warn!("{}", e.to_string());
                warn!("Failed to persist logs to db - will retry on next interval")
            }
        }
    }
}

pub fn persist_cache(
    mut logs_to_persist: VecDeque<LogMessage>,
    use_test_client: bool,
) -> DuckResult<()> {
    // TODO: this is a bit nasty just to be able to test this func - find a better way to do this
    let db_client = if use_test_client {
        get_test_db_client()
    } else {
        get_db_client()
    };
    while logs_to_persist.len() > 0 {
        let msg = logs_to_persist.pop_front().unwrap();
        let mut app = db_client.appender("logstore")?;
        app.append_row(params![
            msg.workflow_id,
            msg.workflow_name,
            msg.workflow_instance_id,
            msg.timestamp_ms,
            msg.level,
            msg.payload,
        ])?
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    use cdktr_db::get_test_db_client;

    #[tokio::test]
    async fn test_persist_cache() {
        let db_client = get_test_db_client();
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

        persist_cache(q.dump().await, true).expect("Failed to persist the cached log messages");

        let mut stmt = db_client.prepare("SELECT * FROM logstore").unwrap();
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
    }
}
