use cdktr_core::{
    exceptions::GenericError,
    get_cdktr_setting,
    zmq_helpers::{get_server_tcp_uri, get_zmq_pub, get_zmq_pull},
};
use log::{debug, info, trace, warn};
use zeromq::{PubSocket, PullSocket, SocketRecv, SocketSend};

use crate::log_manager::model::LogMessage;

/// This module provides the LogManager which is responsible for managing
/// the logging system of the CDKTR application.
/// Each agent will publish log messages to the log manager pull socket,
/// and the log manager will consolidate these messages by workflow ID topics
/// and publish them to the pub socket.
pub struct LogManager {
    pub_socket: PubSocket,
    pull_socket: PullSocket,
}

impl LogManager {
    pub async fn new() -> Result<Self, GenericError> {
        Ok(LogManager {
            pull_socket: get_zmq_pull(&get_server_tcp_uri(
                get_cdktr_setting!(CDKTR_PRINCIPAL_HOST).as_str(),
                get_cdktr_setting!(CDKTR_LOGS_LISTENING_PORT, usize),
            ))
            .await?,
            pub_socket: get_zmq_pub(&get_server_tcp_uri(
                get_cdktr_setting!(CDKTR_PRINCIPAL_HOST).as_str(),
                get_cdktr_setting!(CDKTR_LOGS_PUBLISHING_PORT, usize),
            ))
            .await?,
        })
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
                    trace!(
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
