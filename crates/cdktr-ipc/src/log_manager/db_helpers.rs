use std::time::{Duration, SystemTime};

use cdktr_core::exceptions::GenericError;
use duckdb::Connection;
use log::warn;

use crate::log_manager::model::LogMessage;

pub fn read_logs<'a>(
    db_client: &'a Connection,
    start_timestamp_ms: Option<u64>,
    end_timestamp_ms: Option<u64>,
    workflow_id: Option<&str>,
    workflow_instance_id: Option<&str>,
) -> Result<Vec<LogMessage>, GenericError> {
    let end_timestamp_ms = if let Some(ts) = end_timestamp_ms {
        ts
    } else {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    };
    let start_timestamp_ms = if let Some(ts) = start_timestamp_ms {
        ts
    } else {
        end_timestamp_ms - Duration::from_secs(86400).as_millis() as u64 // default to previous 24 hours of end time
    };
    let mut stmt_str = format!(
        "SELECT * FROM logstore WHERE timestamp_ms >= '{start_timestamp_ms}' AND timestamp_ms < {end_timestamp_ms} "
    );
    if let Some(wf_id) = workflow_id {
        stmt_str.push_str(&format!("AND workflow_id = '{wf_id}' "));
    };
    if let Some(wf_ins_id) = workflow_instance_id {
        stmt_str.push_str(&format!("AND workflow_instance_id = '{wf_ins_id}' "));
    };
    let mut stmt = db_client.prepare(&stmt_str).unwrap();
    let results = stmt
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
        .map_err(|e| GenericError::DBError(e.to_string()))?
        .map(|msg_res| msg_res.map_err(|e| GenericError::DBError(e.to_string())))
        .collect::<Vec<Result<LogMessage, GenericError>>>();
    let mut msgs = Vec::new();
    for res in results {
        match res {
            Ok(msg) => msgs.push(msg),
            Err(e) => {
                warn!("Failed to read msg {:?}", e)
            }
        }
    }
    Ok(msgs)
}

mod tests {
    use cdktr_core::utils::data_structures::AsyncQueue;
    use cdktr_db::DBClient;

    use super::*;
    use crate::log_manager::model::LogMessage;

    #[tokio::test]
    async fn test_read_logs() {
        let db_client = DBClient::new(None).unwrap();

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
        let locked_client = db_client.lock_inner_client().await;
        locked_client
            .execute(
                "INSERT INTO logstore (workflow_id, workflow_name, workflow_instance_id, timestamp_ms, level, payload) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                [
                    &msg1.workflow_id,
                    &msg1.workflow_name,
                    &msg1.workflow_instance_id,
                    &msg1.timestamp_ms.to_string(),
                    &msg1.level,
                    &msg1.payload,
                ],
            )
            .unwrap();
        locked_client
            .execute(
                "INSERT INTO logstore (workflow_id, workflow_name, workflow_instance_id, timestamp_ms, level, payload) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                [
                    &msg2.workflow_id,
                    &msg2.workflow_name,
                    &msg2.workflow_instance_id,
                    &msg2.timestamp_ms.to_string(),
                    &msg2.level,
                    &msg2.payload,
                ],
            )
            .unwrap();

        let messages = read_logs(
            &locked_client,
            Some(0),
            Some(3000000000),
            Some("test_workflow_id"),
            Some("test_workflow_instance_id"),
        )
        .expect("Failed to read logs");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0], msg1);
        assert_eq!(messages[1], msg2);
    }
}
