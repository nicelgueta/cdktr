use std::{error::Error, sync::Arc};
use super::msg::{PrincipalRequest, ClientResponseMessage};
use tokio::sync::Mutex;
use zeromq::{PubSocket, Socket, SocketRecv, SocketSend};


pub async fn start(
    publisher: Arc<Mutex<PubSocket>>,
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
        let msg_res = PrincipalRequest::try_from(zmq_recv.clone());
        if let Ok(cli_msg) = msg_res {
            let response = handle_client_message(
                &publisher,
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
async fn handle_client_message(
    publisher: &Arc<Mutex<PubSocket>>,
    cli_msg: PrincipalRequest
) -> ClientResponseMessage {
    match cli_msg {
        PrincipalRequest::Ping => ClientResponseMessage::Pong,
        // PrincipalRequest::RunTask((ins_id, cmd, args)) => {
        //     let task = Task {
        //         instance_id: ins_id,
        //         command: cmd,
        //         args: Some(args)
        //     };
        //     {
        //         let mut pub_mut = publisher.lock().await;
        //         pub_mut.send(task.to_msg_string().into()).await.unwrap();
        //     }

        //     // return
        //     ClientResponseMessage::Success
        // }
    }
}