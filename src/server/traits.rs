use super::models::{RepReqError, ClientResponseMessage};
use async_trait::async_trait;
use std::error::Error;
use zeromq::{Socket, SocketRecv, SocketSend};
use zeromq::ZmqMessage;

pub trait BaseClientRequestMessage: 
    TryFrom<ZmqMessage, Error = RepReqError> + Send
{
    fn from_zmq_str(s: &str) -> Result<Self, RepReqError> ;
}

/// A standard ZMQ REP server that both the Agent and Principal instances
/// implement
#[async_trait]
pub trait Server<RT>
where
    RT: BaseClientRequestMessage 
{

    /// Method to handle the client request. It returns a tuple of ClientResponseMessage
    /// and a restart flag. This flag is used to determine whether the 
    /// instance should be restarted or not
    async fn handle_client_message(
        &mut self, 
        cli_msg: RT
    ) -> (ClientResponseMessage, bool) ;

    /// Method to run the REP listening loop. This is a default
    /// implementation and is exactly the same for both the Agent
    /// and Principal instances so it is not needed to override this
    /// implmentation
    async fn start(
        &mut self,
        current_host: &str, 
        rep_port: usize,
    )  -> Result<(), Box<dyn Error>> {
        
        println!("SERVER: Starting REP Server on tcp://{}:{}", current_host, rep_port);
        let mut socket = zeromq::RepSocket::new();
        socket
            .bind(&format!("tcp://{}:{}", current_host, rep_port))
            .await
            .expect("Failed to connect");
        println!("SERVER: Successfully connected");
    
        loop {
            let zmq_recv = socket.recv().await?;
            let msg_res: Result<RT, RepReqError> = RT::try_from(
                zmq_recv.clone()
            );
            match msg_res {
                Ok(cli_msg) => {
                    let (response, should_restart) = self.handle_client_message(
                        cli_msg
                    ).await;
                    socket.send(response.into()).await?;
                    if should_restart {
                        // exit the loop in order for the server to be restarted
                        break
                    };
                },
                Err(e) => {
                    let error_msg = e.to_string();
                    println!("SERVER: Invalid message type: {}", error_msg);
                    let response = ClientResponseMessage::ClientError(error_msg);
                    socket.send(response.into()).await?;
                }
            }
        };
        Ok(())
    }
}