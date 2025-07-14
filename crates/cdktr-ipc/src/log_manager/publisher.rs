use log::warn;
use std::env;
use std::time::SystemTime;

use cdktr_core::{
    exceptions::{cdktr_result, GenericError},
    get_cdktr_setting,
    zmq_helpers::{get_server_tcp_uri, get_zmq_push},
};
use zeromq::{PushSocket, SocketSend};

use crate::log_manager::model::LogMessage;

pub struct LogsPublisher {
    workflow_name: String,
    workflow_instance_id: String,
    push_socket: PushSocket,
}

impl LogsPublisher {
    pub async fn new(workflow_name: String, workflow_instance_id: String) -> Self {
        let logs_listen_port = get_cdktr_setting!(CDKTR_LOGS_LISTENING_PORT, usize);
        let prin_host = get_cdktr_setting!(CDKTR_PRINCIPAL_HOST);
        LogsPublisher {
            workflow_name,
            workflow_instance_id,
            push_socket: get_zmq_push(&get_server_tcp_uri(&prin_host, logs_listen_port)).await,
        }
    }

    pub async fn pub_msg(&mut self, level: String, msg: String) -> Result<(), GenericError> {
        let timestamp_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis();
        let msg = LogMessage::new(
            self.workflow_name.clone(),
            self.workflow_instance_id.clone(),
            timestamp_ms,
            level,
            msg,
        );
        cdktr_result(self.push_socket.send(msg.into()).await)
    }
}
