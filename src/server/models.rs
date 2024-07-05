
#[derive(Debug)]
pub struct ClientConversionError {
    pub msg: String
}

#[derive(PartialEq)]
pub enum ClientResponseMessage {
    InvalidMessageType,
    Pong,
    Success,
    Heartbeat(String)
}

pub mod traits {
    use super::{ClientConversionError, ClientResponseMessage};
    use async_trait::async_trait;
    use std::error::Error;
    use zeromq::{Socket, SocketRecv, SocketSend};
    use zeromq::ZmqMessage;

    pub trait BaseClientRequestMessage: TryFrom<ZmqMessage> + Send {
        fn from_zmq_str(s: &str) -> Result<Self, ClientConversionError> ;
    }
    
    /// A standard ZMQ REP server that both the Agent and Principal instances
    /// implement
    #[async_trait]
    pub trait Server<RT> 
    where 
        RT: BaseClientRequestMessage,
        <RT as TryFrom<ZmqMessage>>::Error: Send 
    {

        /// Method to handle the client request. It returns a tuple of ClientResponseMessage
        /// and a restart flag. This flag is used to determine whether the 
        /// instance should be restarted or not
        async fn handle_client_message(
            &self, 
            cli_msg: RT
        ) -> (ClientResponseMessage, bool) ;

        /// Method to run the REP listening loop. This is a default
        /// implementation and is exactly the same for both the Agent
        /// and Principal instances so it is not needed to override this
        /// implmentation
        async fn start(
            &self,
            current_host: &str, 
            rep_port: usize,
        )  -> Result<(), Box<dyn Error>> {
            
            println!("SERVER: Starting REQ/REP Server on tcp://{}:{}", current_host, rep_port);
            let mut socket = zeromq::RepSocket::new();
            socket
                .bind(&format!("tcp://{}:{}", current_host, rep_port))
                .await
                .expect("Failed to connect");
            println!("SERVER: Successfully connected");
        
            loop {
                let zmq_recv = socket.recv().await?;
                let msg_res = RT::try_from(
                    zmq_recv.clone()
                );
                if let Ok(cli_msg) = msg_res {
                    let (response, should_restart) = self.handle_client_message(
                        cli_msg
                    ).await;
                    socket.send(response.into()).await?;
                    if should_restart {
                        // exit the loop in order for the server to be restarted
                        break
                    };
                } else {
                    let zmq_msg_s = String::try_from(zmq_recv).unwrap();
                    println!("SERVER: Invalid message type: {}", zmq_msg_s);
                    let response = ClientResponseMessage::InvalidMessageType;
                    socket.send(response.into()).await?;
                }
            };
            Ok(())
        }
    }

}