use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use zeromq::{ZmqMessage, PubSocket};
use super::{
    parse_zmq_str,
    models::{
        traits::{Server, BaseClientRequestMessage},
        ClientResponseMessage,
        ClientConversionError
    }
};

// TODO: make an extension of AgentRequest
pub enum PrincipalRequest {
    Ping,
}


pub struct PrincipalServer {
    publisher: Arc<Mutex<PubSocket>>
}

impl PrincipalServer {
    pub fn new(publisher: Arc<Mutex<PubSocket>>) -> Self  {
        Self {publisher}
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