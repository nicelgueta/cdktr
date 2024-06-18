use std::error::Error;
use api::{ClientRequestMessage, ClientResponseMessage};
use zeromq::{Socket, SocketRecv, SocketSend};
mod api;

pub struct Server;

impl Server {
    pub async fn start(&self, host: &str, port: usize)  -> Result<(), Box<dyn Error>>{
        println!("SERVER: Starting REQ/REP Server on tcp://{}:{}", host, port);
        let mut socket = zeromq::RepSocket::new();
        socket
            .bind(&format!("tcp://{}:{}", host, port))
            .await
            .expect("Failed to connect");
        println!("SERVER: Successfully connected");
        loop {
            let zmq_recv = socket.recv().await?;
            let msg_res = ClientRequestMessage::try_from(zmq_recv.clone());
            if let Ok(cli_msg) = msg_res {
                let response = self.handle_client_message(cli_msg);
                socket.send(response.into()).await?;
            } else {
                let zmq_msg_s = String::try_from(zmq_recv).unwrap();
                println!("SERVER: Invalid message type: {}", zmq_msg_s);
                let response = ClientResponseMessage::InvalidMessageType;
                socket.send(response.into()).await?;
            }
        }
    }
    fn handle_client_message(&self, cli_msg: ClientRequestMessage) -> ClientResponseMessage {
        match cli_msg {
            ClientRequestMessage::Ping => ClientResponseMessage::Pong,
        }
    }
}