use std::error::Error;
use api::{ClientRequestMessage, ClientResponseMessage};
use zeromq::{Socket, SocketRecv, SocketSend, PubSocket};
use crate::interfaces::Task;
mod api;

pub struct Server {
    publisher: PubSocket
}

impl Server {
    pub fn new() -> Self {
        let publisher = zeromq::PubSocket::new();
        Self {
            publisher
        }
    }
    pub async fn start(&mut self, host: &str, port: usize)  -> Result<(), Box<dyn Error>>{
        let req_rep_port = port+1;
        
        self.publisher
            .connect(
                &format!(
                    "tcp://{}:{}", host, port
                )
            )
            .await
            .expect(&format!(
                "Unable to create publisher on {}:{}", host, port
            ));
            
        println!("SERVER: Starting REQ/REP Server on tcp://{}:{}", host, req_rep_port);
        let mut socket = zeromq::RepSocket::new();
        socket
            .bind(&format!("tcp://{}:{}", host, req_rep_port))
            .await
            .expect("Failed to connect");
        println!("SERVER: Successfully connected");

        loop {
            let zmq_recv = socket.recv().await?;
            let msg_res = ClientRequestMessage::try_from(zmq_recv.clone());
            if let Ok(cli_msg) = msg_res {
                let response = self.handle_client_message(cli_msg).await;
                socket.send(response.into()).await?;

            } else {
                let zmq_msg_s = String::try_from(zmq_recv).unwrap();
                println!("SERVER: Invalid message type: {}", zmq_msg_s);
                let response = ClientResponseMessage::InvalidMessageType;
                socket.send(response.into()).await?;
            }
        }
    }
    async fn handle_client_message(&mut self, cli_msg: ClientRequestMessage) -> ClientResponseMessage {
        match cli_msg {
            ClientRequestMessage::Ping => ClientResponseMessage::Pong,
            ClientRequestMessage::Echo(args) => {
                let task = Task {
                    command: "echo".to_string(),
                    args: Some(args)
                };
                self.publisher.send(
                    task.to_msg_string().into()
                ).await.expect("Failed to trigger echo");
                ClientResponseMessage::Success
            }
        }
    }
}