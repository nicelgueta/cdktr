pub mod client;
mod db_helpers;
pub mod manager;
pub mod model;
pub mod persister;
pub mod publisher;

#[cfg(test)]
mod tests {
    use cdktr_core::exceptions::GenericError;
    use regex::Regex;
    use std::time::{Duration, SystemTime};
    use tokio::task::JoinSet;

    use super::{
        client::LogsClient, manager::LogManager, model::LogMessage, publisher::LogsPublisher,
    };

    fn get_time() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
    #[tokio::test]
    async fn test_log_message_format() {
        let timestamp = get_time();
        let log_msg = LogMessage::new(
            "test_workflow_id".to_string(),
            "test_workflow".to_string(),
            "jumping-monkey-0".to_string(),
            timestamp,
            "INFO".to_string(),
            "This is a test log message".to_string(),
        );
        let formatted = log_msg.format();
        assert!(formatted.contains("jumping-monkey-0"));
        assert!(formatted.contains("INFO"));
        assert!(formatted.contains("This is a test log message"));
    }

    #[tokio::test]
    async fn test_log_message_format_full() {
        let timestamp = get_time();
        let log_msg = LogMessage::new(
            "test_workflow_id".to_string(),
            "Test Workflow".to_string(),
            "jumping-monkey-0".to_string(),
            timestamp,
            "INFO".to_string(),
            "This is a test log message".to_string(),
        );
        let formatted = log_msg.format_full();
        assert!(formatted.contains("Test Workflow/jumping-monkey-0"));
        assert!(formatted.contains("INFO"));
        assert!(formatted.contains("This is a test log message"));
    }

    #[tokio::test]
    async fn test_log_manager_start_e2e() -> Result<(), GenericError> {
        let test_workflow_id = "test_workflow_id";
        let test_workflow_name = "Test Workflow";
        let test_workflow_instance_id = "jumping-monkey-0";

        let mut join_set: JoinSet<Result<(), GenericError>> = JoinSet::new();

        join_set.spawn(async move {
            let mut log_manager = LogManager::new().await?;
            log_manager.start().await;
            Ok(())
        });

        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        // spawn process to listen to messages from the log manager
        join_set.spawn(async move {
            let mut logs_client =
                LogsClient::new("test_client".to_string(), test_workflow_id).await?;
            let _ = logs_client
                .listen(tx, Some(Duration::from_millis(4000)))
                .await
                .is_err();
            Ok(())
        });

        join_set.spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let mut logs_publisher = LogsPublisher::new(
                test_workflow_id.to_string(),
                test_workflow_name.to_string(),
                test_workflow_instance_id.to_string(),
            )
            .await
            .unwrap();
            let _ = logs_publisher
                .pub_msg("INFO".to_string(), "test message 1".to_string())
                .await;
            let _ = logs_publisher
                .pub_msg("DEBUG".to_string(), "test message 2".to_string())
                .await;
            Ok(())
        });
        tokio::time::sleep(Duration::from_secs(3)).await;
        let mut msgs = Vec::new();
        while let Some(msg) = rx.recv().await {
            msgs.push(msg.format_full());
        }
        dbg!(&msgs);
        let regs = vec![
            Regex::new(r"^\[\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+[\+\-]\d{2}:\d{2} INFO\] \[Test Workflow/jumping-monkey-0\] test message 1$").unwrap(),
            Regex::new(r"^\[\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+[\+\-]\d{2}:\d{2} DEBUG\] \[Test Workflow/jumping-monkey-0\] test message 2$").unwrap(),
        ];
        for (i, reg) in regs.iter().enumerate() {
            let res = msgs[i].as_str();
            assert!(reg.is_match(res));
        }
        Ok(())
    }
}
