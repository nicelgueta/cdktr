use cdktr_core::{
    exceptions::{cdktr_result, GenericError, ZMQParseError},
    get_cdktr_setting,
    models::ZMQArgs,
    zmq_helpers::{get_server_tcp_uri, get_zmq_pub, get_zmq_pull, get_zmq_push, get_zmq_sub},
};
use log::{debug, info, warn};
use std::{
    env,
    time::{Duration, SystemTime},
};
use tokio::sync::mpsc::Sender;
use zeromq::{PubSocket, PullSocket, PushSocket, SocketRecv, SocketSend, SubSocket, ZmqMessage};

/// This module provides the LogManager which is responsible for managing
/// the logging system of the CDKTR application.
/// Each agent will publish log messages to the log manager rep socket,
/// and the log manager will consolidate these messages by worflow ID topics
/// and publish them to the pub socket.
struct LogManager {
    pub_socket: PubSocket,
    pull_socket: PullSocket,
}

impl LogManager {
    pub async fn new() -> Self {
        LogManager {
            pull_socket: get_zmq_pull(&get_server_tcp_uri(
                get_cdktr_setting!(CDKTR_PRINCIPAL_HOST).as_str(),
                get_cdktr_setting!(CDKTR_LOGS_LISTENING_PORT, usize),
            ))
            .await,
            pub_socket: get_zmq_pub(&get_server_tcp_uri(
                get_cdktr_setting!(CDKTR_PRINCIPAL_HOST).as_str(),
                get_cdktr_setting!(CDKTR_LOGS_PUBLISHING_PORT, usize),
            ))
            .await,
        }
    }

    pub async fn start(&mut self) {
        info!("LogManager started, listening for log messages from agents...");
        loop {
            match self.pull_socket.recv().await {
                Ok(msg) => {
                    let log_message: LogMessage = match LogMessage::try_from(msg) {
                        Ok(log_msg) => log_msg,
                        Err(e) => {
                            debug!("Failed to parse log message: {}", e);
                            dbg!(e);
                            continue;
                        }
                    };
                    debug!(
                        "Received log message on topic {}: {}",
                        &log_message.workflow_instance_id, &log_message.payload
                    );
                    // Publish the log message to the pub socket
                    if let Err(e) = self.pub_socket.send(log_message.into()).await {
                        warn!("Failed to publish log message: {}", e);
                    }
                }
                Err(e) => {
                    warn!("Error receiving message: {}", e);
                }
            }
        }
    }
}

pub struct LogsPublisher {
    workflow_instance_id: String,
    push_socket: PushSocket,
}

impl LogsPublisher {
    pub async fn new(workflow_instance_id: String) -> Self {
        let logs_listen_port = get_cdktr_setting!(CDKTR_LOGS_LISTENING_PORT, usize);
        let prin_host = get_cdktr_setting!(CDKTR_PRINCIPAL_HOST);
        LogsPublisher {
            workflow_instance_id,
            push_socket: get_zmq_push(&get_server_tcp_uri(&prin_host, logs_listen_port)).await,
        }
    }

    pub async fn pub_msg(&mut self, level: String, msg: String) -> Result<(), GenericError> {
        let timestamp_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis();
        let msg = LogMessage::new(self.workflow_instance_id.clone(), timestamp_ms, level, msg);
        cdktr_result(self.push_socket.send(msg.into()).await)
    }
}
pub struct LogsClient {
    client_name: String,
    sub_socket: SubSocket,
}

impl LogsClient {
    pub async fn new(client_name: String, topic: &str) -> Self {
        let logs_pub_port = get_cdktr_setting!(CDKTR_LOGS_PUBLISHING_PORT, usize);
        let prin_host = get_cdktr_setting!(CDKTR_PRINCIPAL_HOST);
        LogsClient {
            client_name,
            sub_socket: get_zmq_sub(&get_server_tcp_uri(&prin_host, logs_pub_port), topic).await,
        }
    }

    pub async fn listen(
        &mut self,
        tx: Sender<LogMessage>,
        loop_timeout: Option<Duration>,
    ) -> Result<(), GenericError> {
        if let Some(to) = loop_timeout {
            if let Err(_e) = tokio::time::timeout(to, self.listen_loop(tx)).await {
                return Err(GenericError::TimeoutError);
            } else {
                Ok(())
            }
        } else {
            self.listen_loop(tx).await
        }
    }

    async fn listen_loop(&mut self, tx: Sender<LogMessage>) -> Result<(), GenericError> {
        loop {
            let msg = LogMessage::try_from(cdktr_result(self.sub_socket.recv().await)?)?;
            match tx.send(msg).await {
                Ok(_) => (),
                Err(e) => {
                    warn!(
                        "Failed to send received log manager to {}",
                        &self.client_name
                    )
                }
            }
        }
    }
}
pub struct LogMessage {
    workflow_instance_id: String,
    timestamp_ms: u128,
    level: String,
    payload: String,
}

impl LogMessage {
    pub fn new(
        workflow_instance_id: String,
        timestamp_ms: u128,
        level: String,
        payload: String,
    ) -> Self {
        LogMessage {
            workflow_instance_id,
            timestamp_ms,
            level,
            payload,
        }
    }
    pub fn format(&self) -> String {
        let timestring = chrono::DateTime::from_timestamp_millis(self.timestamp_ms as i64)
            .unwrap()
            .to_rfc3339();
        format!(
            "{} [{} {}] {}",
            self.workflow_instance_id, timestring, self.level, self.payload
        )
    }
}

impl TryFrom<ZmqMessage> for LogMessage {
    type Error = GenericError;
    fn try_from(msg: ZmqMessage) -> Result<Self, Self::Error> {
        let mut zmq_args: ZMQArgs = msg.into();
        if zmq_args.len() < 4 {
            return Err(GenericError::ZMQParseError(ZMQParseError::ParseError(
                "LogMessage must have at least 4 parts: topic, timestamp, level and payload"
                    .to_string(),
            )));
        }
        Ok(LogMessage {
            workflow_instance_id: zmq_args.next().unwrap(),
            timestamp_ms: cdktr_result(zmq_args.next().unwrap().parse())?,
            level: zmq_args.next().unwrap(),
            payload: zmq_args.next().unwrap(),
        })
    }
}

impl Into<ZmqMessage> for LogMessage {
    fn into(self) -> ZmqMessage {
        ZmqMessage::from(format!(
            "{}|{}|{}|{}",
            self.workflow_instance_id, self.timestamp_ms, self.level, self.payload
        ))
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;
    use std::time::Duration;

    use super::*;
    use cdktr_core::zmq_helpers::get_zmq_push;
    use tokio::{task::JoinSet, time::timeout};
    use zeromq::{SocketSend, ZmqMessage};

    fn get_time() -> u128 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    }
    #[tokio::test]
    async fn test_log_message_format() {
        let timestamp = get_time();
        let log_msg = LogMessage::new(
            "test_workflow".to_string(),
            timestamp,
            "INFO".to_string(),
            "This is a test log message".to_string(),
        );
        let formatted = log_msg.format();
        assert!(formatted.contains("test_workflow"));
        assert!(formatted.contains("INFO"));
        assert!(formatted.contains("This is a test log message"));
    }

    #[tokio::test]
    async fn test_log_manager_start_e2e() {
        let test_topic = "test_workflow";

        let mut join_set = JoinSet::new();

        join_set.spawn(async move {
            let mut log_manager = LogManager::new().await;
            log_manager.start().await;
        });

        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        // spawn process to listen to messages from the log manager
        join_set.spawn(async move {
            let mut logs_client = LogsClient::new("test_client".to_string(), test_topic).await;
            let _ = logs_client
                .listen(tx, Some(Duration::from_millis(4000)))
                .await
                .is_err();
        });

        join_set.spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let mut logs_publisher = LogsPublisher::new(test_topic.to_string()).await;
            let _ = logs_publisher
                .pub_msg("INFO".to_string(), "test message 1".to_string())
                .await
                .unwrap();
            let _ = logs_publisher
                .pub_msg("DEBUG".to_string(), "test message 2".to_string())
                .await
                .unwrap();
        });
        tokio::time::sleep(Duration::from_secs(3)).await;
        let mut msgs = Vec::new();
        while let Some(msg) = rx.recv().await {
            msgs.push(msg.format());
        }

        let regs = vec![
            Regex::new(r"^test_workflow \[\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+[\+\-]\d{2}:\d{2} INFO\] test message 1$").unwrap(),
            Regex::new(r"^test_workflow \[\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+[\+\-]\d{2}:\d{2} DEBUG\] test message 2$").unwrap(),
        ];
        for (i, reg) in regs.iter().enumerate() {
            assert!(reg.is_match(msgs[i].as_str()));
        }
    }
}
