use crate::exceptions::GenericError;
use crate::models::ZMQArgs;
use crate::zmq_helpers::{get_server_tcp_uri, get_zmq_rep, send_recv_with_timeout};

use super::models::{ClientResponseMessage, RepReqError};
use async_trait::async_trait;
use log::{info, trace};
use std::error::Error;
use std::time::Duration;
use zeromq::ZmqMessage;
use zeromq::{Socket, SocketRecv, SocketSend};

/// A standard ZMQ REP server that both the Agent and Principal instances
/// implement
#[async_trait]
pub trait Server<RT>
where
    RT: TryFrom<ZmqMessage, Error = RepReqError> + Send,
{
    /// Method to handle the client request. It returns a tuple of ClientResponseMessage
    /// and a restart flag. This flag is used to determine whether the
    /// instance should be restarted or not
    async fn handle_client_message(&mut self, cli_msg: RT) -> (ClientResponseMessage, usize);

    /// Method to run the REP listening loop. This is a default
    /// implementation and is exactly the same for both the Agent
    /// and Principal instances so it is not needed to override this
    /// implmentation.
    async fn start(
        &mut self,
        current_host: &str,
        rep_port: usize,
    ) -> Result<usize, Box<dyn Error>> {
        info!(
            "SERVER: Starting REP Server on tcp://{}:{}",
            current_host, rep_port
        );
        let mut rep_socket = get_zmq_rep(&get_server_tcp_uri(current_host, rep_port)).await;
        info!("SERVER: Successfully connected");

        let exit_code = loop {
            let zmq_recv = rep_socket.recv().await?;
            let msg_res: Result<RT, RepReqError> = RT::try_from(zmq_recv.clone());
            match msg_res {
                Ok(cli_msg) => {
                    let (response, exit_code) = self.handle_client_message(cli_msg).await;
                    rep_socket.send(response.into()).await?;
                    if exit_code > 0 {
                        // received a non-zero exit code from the message handling function
                        // which means the server should perform some other kind of action
                        // above the client/request loop so loop should be exited
                        break exit_code;
                    };
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    let response = ClientResponseMessage::ClientError(error_msg);
                    rep_socket.send(response.into()).await?;
                }
            }
        };
        Ok(exit_code)
    }
}

pub struct APIMeta {
    action: String,
    description: String,
}
impl APIMeta {
    pub const fn new(action: String, description: String) -> Self {
        Self {
            action,
            description,
        }
    }
    pub fn try_to_api<T>(&self) -> Result<T, RepReqError>
    where
        T: API,
        <T as TryFrom<String>>::Error: Into<RepReqError>,
    {
        T::try_from(self.action.clone()).map_err(Into::into)
    }
}

/// The API trait defines the interface for the ZMQ-based APIs that external modules and systems
/// can leverage to communicate with CDKTR. The APIs are also used internally between different components
///  
#[async_trait]
pub trait API: Into<ZmqMessage> + TryFrom<ZmqMessage> + TryFrom<String> + TryFrom<ZMQArgs> {
    fn get_meta(&self) -> Vec<APIMeta>;
    /// Convert the message to a string
    fn to_string(&self) -> String;

    /// Default implementation for sending the message to a destination REP socket within CDKTR
    async fn send(
        self,
        tcp_uri: &str,
        timeout: Duration,
    ) -> Result<ClientResponseMessage, GenericError> {
        trace!("Pinging API @ {} with msg: {}", tcp_uri, self.to_string());
        let zmq_m = send_recv_with_timeout(tcp_uri.to_string(), self.into(), timeout).await?;
        Ok(ClientResponseMessage::from(zmq_m))
    }
}
