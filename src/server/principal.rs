use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use zeromq::{ZmqMessage, PubSocket, Socket};
use super::{
    parse_zmq_str,
    traits::{Server, BaseClientRequestMessage},
    models::{
        ClientResponseMessage,
        ClientConversionError
    }
};

// TODO: make an extension of AgentRequest
pub enum PrincipalRequest {
    Ping,
}


pub struct PrincipalServer {
    publisher: Arc<Mutex<PubSocket>>,
    req: zeromq::ReqSocket
}

impl PrincipalServer {
    pub fn new(publisher: Arc<Mutex<PubSocket>>) -> Self  {
        let req = zeromq::ReqSocket::new();
        Self { publisher, req }
    }
}

#[async_trait]
impl Server<PrincipalRequest> for PrincipalServer {

    async fn handle_client_message(
        &mut self, 
        cli_msg: PrincipalRequest
    ) -> (ClientResponseMessage, bool) {
        match cli_msg {
            PrincipalRequest::Ping => (ClientResponseMessage::Pong, false),
        }
    }
}


#[async_trait]
impl BaseClientRequestMessage for PrincipalRequest {
    fn from_zmq_str(s: &str) -> Result<PrincipalRequest, ClientConversionError> {
        let (msg_type, args) = parse_zmq_str(s);
        match msg_type {
            // "GET_TASKS" => Ok(Self::GetTasks),
            "PING" => Ok(Self::Ping),
            _ => Err(ClientConversionError::new(format!("Unrecognised server message: {}", msg_type)))
        }
    }
}
impl TryFrom<ZmqMessage> for PrincipalRequest {
    type Error = ClientConversionError;
    fn try_from(value: ZmqMessage) -> Result<Self, Self::Error> {
        let zmq_msg_s = String::try_from(value).expect(
            "Unable to convert ZMQ Client message to String"
        );
        Self::from_zmq_str(&zmq_msg_s)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_principal_request_from_zmq_str_all_happy(){
        const ALL_HAPPIES: [&str; 1] = [
            "PING",
        ];
        for zmq_s in ALL_HAPPIES {
            let res = PrincipalRequest::from_zmq_str(zmq_s);
            assert!(res.is_ok())
        }
    }

    #[tokio::test]
    async fn test_handle_cli_message_all_happy(){
        let test_params: [(&str, ClientResponseMessage, bool); 1] = [
            ("PING", ClientResponseMessage::Pong, false),
        ];
        let fake_publisher = Arc::new(Mutex::new(PubSocket::new()));
        let mut server = PrincipalServer::new(fake_publisher);
        for (
            zmq_s, response, restart_flag
        ) in test_params {
            let ar = PrincipalRequest::from_zmq_str(zmq_s).expect(
                "Should be able to unwrap the agent from ZMQ command"
            );
            let (resp, flag) = server.handle_client_message(
                ar
            ).await;
            println!("Testing {zmq_s}");
            assert_eq!(response, resp);
            assert_eq!(flag, restart_flag);
        }

    }
}