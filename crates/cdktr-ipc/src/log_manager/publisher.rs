use std::collections::VecDeque;
use std::time::SystemTime;

use cdktr_core::{
    exceptions::GenericError,
    get_cdktr_setting,
    zmq_helpers::{get_server_tcp_uri, get_zmq_push},
};
use zeromq::{PushSocket, Socket, SocketSend};

use crate::log_manager::model::LogMessage;

pub struct TaskLogger<'a, 'b> {
    task_name: &'a str,
    task_instance_id: &'a str,
    publisher: &'b mut LogsPublisher,
}

impl<'a, 'b> TaskLogger<'a, 'b> {
    fn new(
        publisher: &'b mut LogsPublisher,
        task_name: &'a str,
        task_instance_id: &'a str,
    ) -> Self {
        Self {
            publisher,
            task_name,
            task_instance_id,
        }
    }

    pub async fn info(&mut self, msg: &str) {
        self.publisher
            .pub_msg("INFO", self.task_name, self.task_instance_id, msg)
            .await
    }

    pub async fn warn(&mut self, msg: &str) {
        self.publisher
            .pub_msg("WARNING", self.task_name, self.task_instance_id, msg)
            .await
    }

    pub async fn error(&mut self, msg: &str) {
        self.publisher
            .pub_msg("ERROR", self.task_name, self.task_instance_id, msg)
            .await
    }
}

pub struct LogsPublisher {
    prin_host: String,
    logs_listen_port: usize,
    workflow_id: String,
    workflow_name: String,
    workflow_instance_id: String,
    push_socket: PushSocket,
    log_queue: VecDeque<LogMessage>,
}

impl LogsPublisher {
    pub async fn new(
        workflow_id: String,
        workflow_name: String,
        workflow_instance_id: String,
    ) -> Result<Self, GenericError> {
        let logs_listen_port = get_cdktr_setting!(CDKTR_LOGS_LISTENING_PORT, usize);
        let prin_host = get_cdktr_setting!(CDKTR_PRINCIPAL_HOST);
        let url = &get_server_tcp_uri(&prin_host, logs_listen_port);
        Ok(LogsPublisher {
            prin_host,
            logs_listen_port,
            workflow_id,
            workflow_name,
            workflow_instance_id,
            push_socket: get_zmq_push(url).await?,
            log_queue: VecDeque::new(),
        })
    }

    pub async fn get_task_logger<'a, 'b>(
        &'b mut self,
        task_name: &'a str,
        task_instance_id: &'a str,
    ) -> TaskLogger<'a, 'b> {
        TaskLogger::new(self, task_name, task_instance_id)
    }

    /// Publishes a log message to the principal server
    /// If it fails to send then it will store it in a local queue
    /// and attempt to resend it next time a new message is published
    pub async fn pub_msg(
        &mut self,
        level: &str,
        task_name: &str,
        task_instance_id: &str,
        msg: &str,
    ) {
        let _ = self.check_and_clear_local_messages().await;
        let timestamp_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis() as u64;
        let log_msg: LogMessage = LogMessage::new(
            self.workflow_id.clone(),
            self.workflow_name.clone(),
            self.workflow_instance_id.clone(),
            task_name.to_string(),
            task_instance_id.to_string(),
            timestamp_ms,
            level.to_string(),
            msg.to_string(),
        );
        match self.push_socket.send(log_msg.into()).await {
            // failed to push to socket so log internally
            // needs to create msg again
            Err(_e) => self.log_queue.push_back(LogMessage::new(
                self.workflow_id.clone(),
                self.workflow_name.clone(),
                self.workflow_instance_id.clone(),
                task_name.to_string(),
                task_instance_id.to_string(),
                timestamp_ms,
                level.to_string(),
                msg.to_string(),
            )),
            Ok(()) => (),
        }
    }

    /// Check if there are any local messages to send and attempt to send them
    /// If there are still messages left then they will remain in the local queue
    /// and will be attempted to be sent next time a new message is published
    async fn check_and_clear_local_messages(&mut self) -> Result<(), GenericError> {
        if self.log_queue.len() > 0 {
            self.attempt_reconnect().await?;
            while self.log_queue.len() > 0 {
                let log_msg = self
                    .log_queue
                    .pop_front()
                    .expect("Message queue is > 0 but pop front fails");
                match self.push_socket.send(log_msg.clone().into()).await {
                    Ok(()) => (),
                    Err(_e) => self.log_queue.push_front(log_msg),
                }
            }
        }
        Ok(())
    }
    async fn attempt_reconnect(&mut self) -> Result<(), GenericError> {
        // Close the old socket before creating a new one to prevent file descriptor leaks
        let old_socket = std::mem::replace(
            &mut self.push_socket,
            get_zmq_push(&get_server_tcp_uri(&self.prin_host, self.logs_listen_port)).await?,
        );
        let close_errors = old_socket.close().await;
        if !close_errors.is_empty() {
            log::warn!("Errors closing old push socket during reconnect: {:?}", close_errors);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // TODO
    #[tokio::test]
    async fn test_pub_msg_success() {}

    #[tokio::test]
    async fn test_pub_msg_failure_queues_message() {}

    #[tokio::test]
    async fn test_check_and_clear_local_messages_sends_queued() {}

    #[tokio::test]
    async fn test_attempt_reconnect_replaces_socket() {}
}
