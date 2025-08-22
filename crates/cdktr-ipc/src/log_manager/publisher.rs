use log::warn;
use std::time::SystemTime;
use std::{collections::VecDeque, env};

use cdktr_core::{
    exceptions::{cdktr_result, GenericError},
    get_cdktr_setting,
    zmq_helpers::{get_server_tcp_uri, get_zmq_push},
};
use zeromq::{PushSocket, SocketSend};

use crate::log_manager::model::LogMessage;

pub struct LogsPublisher {
    prin_host: String,
    logs_listen_port: usize,
    workflow_name: String,
    workflow_instance_id: String,
    push_socket: PushSocket,
    log_queue: VecDeque<LogMessage>,
}

impl LogsPublisher {
    pub async fn new(
        workflow_name: String,
        workflow_instance_id: String,
    ) -> Result<Self, GenericError> {
        let logs_listen_port = get_cdktr_setting!(CDKTR_LOGS_LISTENING_PORT, usize);
        let prin_host = get_cdktr_setting!(CDKTR_PRINCIPAL_HOST);
        let url = &get_server_tcp_uri(&prin_host, logs_listen_port);
        Ok(LogsPublisher {
            prin_host,
            logs_listen_port,
            workflow_name,
            workflow_instance_id,
            push_socket: get_zmq_push(url).await?,
            log_queue: VecDeque::new(),
        })
    }

    pub async fn pub_msg(&mut self, level: String, msg: String) {
        let timestamp_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis();
        let log_msg: LogMessage = LogMessage::new(
            self.workflow_name.clone(),
            self.workflow_instance_id.clone(),
            timestamp_ms,
            level.clone(),
            msg.clone(),
        );
        match self.push_socket.send(log_msg.into()).await {
            // failed to push to socket so log internally
            // needs to create msg again
            Err(e) => self.log_queue.push_back(LogMessage::new(
                self.workflow_name.clone(),
                self.workflow_instance_id.clone(),
                timestamp_ms,
                level,
                msg,
            )),
            Ok(()) => (),
        }
    }
    pub async fn attempt_reconnect(&mut self) -> Result<(), GenericError> {
        self.push_socket =
            get_zmq_push(&get_server_tcp_uri(&self.prin_host, self.logs_listen_port)).await?;
        Ok(())
    }
}
