use async_trait::async_trait;
use cdktr_api::models::ClientResponseMessage;
use cdktr_core::exceptions::GenericError;
use cdktr_core::zmq_helpers::{
    get_router_request, get_router_response, get_server_tcp_uri, get_zmq_rep,
};
use log::info;

use zeromq::{SocketRecv, SocketSend};

/// A standard ZMQ REP server that both the Agent and Principal instances
/// implement
#[async_trait]
pub trait Server<'a, RT>
where
    RT: TryFrom<String, Error = GenericError> + Send,
{
    /// Method to handle the client request. It returns a tuple of ClientResponseMessage
    /// and a restart flag. This flag is used to determine whether the
    /// instance should be restarted or not
    async fn handle_client_message(&mut self, cli_msg: RT) -> (ClientResponseMessage, usize);

    /// Method to run the REP listening loop. This is a default
    /// implementation and is exactly the same for both the Agent
    /// and Principal instances so it is not needed to override this
    /// implmentation.
    async fn start(&mut self, current_host: &str, rep_port: usize) -> Result<usize, GenericError> {
        info!(
            "SERVER: Starting REP Server on tcp://{}:{}",
            current_host, rep_port
        );
        let mut rep_socket = get_zmq_rep(&get_server_tcp_uri(current_host, rep_port)).await?;
        info!("SERVER: Successfully connected");

        let exit_code = loop {
            let msg = rep_socket
                .recv()
                .await
                .map_err(|e| GenericError::ZMQError(e.to_string()))?;
            let (identity, empty, payload) =
                get_router_request(msg).map_err(|e| GenericError::ZMQError(e.to_string()))?;
            let msg_res: Result<RT, GenericError> = RT::try_from(payload);
            match msg_res {
                Ok(cli_msg) => {
                    let (response, exit_code) = self.handle_client_message(cli_msg).await;
                    let resp = get_router_response(identity, empty, response.into())
                        .map_err(|e| GenericError::ZMQError(e.to_string()))?;
                    let _ = rep_socket.send(resp).await;
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
                    let resp = get_router_response(identity, empty, response.into())
                        .map_err(|e| GenericError::ZMQError(e.to_string()))?;
                    rep_socket
                        .send(resp)
                        .await
                        .map_err(|e| GenericError::ZMQError(e.to_string()))?;
                }
            }
        };
        Ok(exit_code)
    }
}
