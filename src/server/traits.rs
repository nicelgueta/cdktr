use crate::hub::InstanceType;

use super::models::{ClientResponseMessage, RepReqError};
use async_trait::async_trait;
use log::info;
use std::error::Error;
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
        let mut socket = zeromq::RepSocket::new();
        socket
            .bind(&format!("tcp://{}:{}", current_host, rep_port))
            .await
            .expect("Failed to connect");
        info!("SERVER: Successfully connected");

        let exit_code = loop {
            let zmq_recv = socket.recv().await?;
            let msg_res: Result<RT, RepReqError> = RT::try_from(zmq_recv.clone());
            match msg_res {
                Ok(cli_msg) => {
                    let (response, exit_code) = self.handle_client_message(cli_msg).await;
                    socket.send(response.into()).await?;
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
                    socket.send(response.into()).await?;
                }
            }
        };
        Ok(exit_code)
    }
}
