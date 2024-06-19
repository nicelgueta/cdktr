use std::error::Error;
use super::msg::{AgentRequest, ClientResponseMessage};
use zeromq::{Socket, SocketRecv, SocketSend};
// use crate::interfaces::Task;


pub async fn start(
    current_host: &str, 
    rep_port: usize
)  -> Result<(), Box<dyn Error>>{
    
    println!("SERVER: Starting REQ/REP Server on tcp://{}:{}", current_host, rep_port);
    let mut socket = zeromq::RepSocket::new();
    socket
        .bind(&format!("tcp://{}:{}", current_host, rep_port))
        .await
        .expect("Failed to connect");
    println!("SERVER: Successfully connected");

    loop {
        let zmq_recv = socket.recv().await?;
        let msg_res = AgentRequest::try_from(zmq_recv.clone());
        if let Ok(cli_msg) = msg_res {
            let response = handle_client_message(
                cli_msg
            ).await;
            socket.send(response.into()).await?;

        } else {
            let zmq_msg_s = String::try_from(zmq_recv).unwrap();
            println!("SERVER: Invalid message type: {}", zmq_msg_s);
            let response = ClientResponseMessage::InvalidMessageType;
            socket.send(response.into()).await?;
        }
    }
}
async fn handle_client_message(cli_msg: AgentRequest) -> ClientResponseMessage {
    match cli_msg {
        AgentRequest::Ping => ClientResponseMessage::Pong,
        // AgentRequest::Echo(args) => {
        //     let task = Task {
        //         command: "echo".to_string(),
        //         args: Some(args)
        //     };
        //     // handle in some way

        //     // return
        //     ClientResponseMessage::Success
        // }
    }
}