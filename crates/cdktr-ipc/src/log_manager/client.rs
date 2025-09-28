use std::env;
use std::time::Duration;

use cdktr_core::{
    exceptions::{GenericError, cdktr_result},
    get_cdktr_setting,
    zmq_helpers::{get_server_tcp_uri, get_zmq_sub},
};
use log::warn;
use tokio::sync::mpsc::Sender;
use zeromq::{SocketRecv, SubSocket};

use crate::log_manager::model::LogMessage;

pub struct LogsClient {
    client_name: String,
    sub_socket: SubSocket,
}

impl LogsClient {
    /// Creates a new instance of the logs client
    /// Args:
    ///     client_name: name of the client instance
    ///     topic: id of the workflow to subscribe to.
    ///         This should be the file stem of the workflow yml, eg: <my_workflow.yml> (my_workflow)
    ///         Will subscribe to all running instances of the same workflow
    pub async fn new(client_name: String, topic: &str) -> Result<Self, GenericError> {
        let logs_pub_port = get_cdktr_setting!(CDKTR_LOGS_PUBLISHING_PORT, usize);
        let prin_host = get_cdktr_setting!(CDKTR_PRINCIPAL_HOST);
        Ok(LogsClient {
            client_name,
            sub_socket: get_zmq_sub(&get_server_tcp_uri(&prin_host, logs_pub_port), topic).await?,
        })
    }

    pub async fn listen(
        &mut self,
        tx: Sender<LogMessage>,
        loop_timeout: Option<Duration>,
    ) -> Result<(), GenericError> {
        if let Some(to) = loop_timeout {
            if let Err(_e) = tokio::time::timeout(to, self.listen_loop(tx)).await {
                return Err(GenericError::PrincipalTimeoutError);
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
                Err(_e) => {
                    warn!(
                        "Failed to send received log manager to {}",
                        &self.client_name
                    )
                }
            }
        }
    }
}
