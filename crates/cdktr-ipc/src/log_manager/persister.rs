use std::{collections::VecDeque, thread, time::Duration};

use crate::log_manager::model::LogMessage;
use cdktr_core::exceptions::{cdktr_result, GenericError};
use duckdb::{params, Connection, Result as DuckResult};
use log::warn;
use tokio::time::{sleep_until, Instant};
use zeromq::{SocketRecv, SubSocket};

// write logs to the database every 30 seconds
static CACHE_PERSISTENCE_INTERVAL_MS: u64 = 30_000;

/// The logs persister loop is a separate component that reads logs from the
/// logs manager pub socket and stores them in duckdb
pub struct LogsPersister<'a> {
    client: &'a Connection,
    logs_cache: VecDeque<LogMessage>,
}
impl<'a> LogsPersister<'a> {
    pub fn new(client: &'a Connection) -> Result<Self, GenericError> {
        client
            .execute(
                "
            create table IF NOT EXISTS logstore
            (
                workflow_id TEXT,
                workflow_name TEXT,
                workflow_instance_id TEXT,
                timestamp_ms BIGINT,
                level TEXT,
                payload TEXT,
            );",
                params![],
            )
            .map_err(|e| GenericError::DBError(e.to_string()))?;
        Ok(LogsPersister {
            client,
            logs_cache: VecDeque::new(),
        })
    }
    pub async fn start_listener(
        &mut self,
        logs_sub_socket: &mut SubSocket,
    ) -> Result<(), GenericError> {
        loop {
            let log_msg = cdktr_result(logs_sub_socket.recv().await)?;
            self.add_msg(LogMessage::try_from(log_msg)?);
        }
    }
    pub async fn start_persistence_loop(&mut self) {
        loop {
            sleep_until(Instant::now() + Duration::from_millis(CACHE_PERSISTENCE_INTERVAL_MS))
                .await;
            match self.persist_cache() {
                Ok(()) => (),
                Err(e) => {
                    warn!("{}", e.to_string());
                    warn!("Failed to persist logs to db - will retry on next interval")
                }
            }
        }
    }
    pub fn add_msg(&mut self, msg: LogMessage) {
        self.logs_cache.push_back(msg);
    }
    pub fn persist_cache(&mut self) -> DuckResult<()> {
        let mut app = self.client.appender("logstore")?;
        while self.logs_cache.len() > 0 {
            let msg = self.logs_cache.pop_front().unwrap();
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
    pub fn msg_count(&self) -> usize {
        self.logs_cache.len()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use cdktr_db::get_test_db_client;

    #[test]
    fn test_add_messages() {
        let db_cli = get_test_db_client();
        let mut lpers = LogsPersister::new(&db_cli).unwrap();
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
        lpers.add_msg(msg1);
        lpers.add_msg(msg2);

        assert_eq!(lpers.msg_count(), 2);
    }

    #[test]
    fn test_persist_cache() {
        let db_client = get_test_db_client();
        let mut lpers = LogsPersister::new(&db_client).unwrap();
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
        lpers.add_msg(msg1);
        lpers.add_msg(msg2);

        lpers
            .persist_cache()
            .expect("Failed to persist the cached log messages");

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
