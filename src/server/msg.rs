
use zeromq::ZmqMessage;
use super::models::{
    ClientResponseMessage,
    ClientConversionError, 
    traits::BaseClientRequestMessage
};
use async_trait::async_trait;

impl ClientConversionError {
    pub fn new(msg: String) -> Self {
        ClientConversionError {msg}
    }
    pub fn to_string(&self) -> String {
        self.msg.clone()
    }
}

pub enum AgentRequest{
    Ping,
    Heartbeat,
    Reconnect
}

// TODO: make an extension of AgentRequest
pub enum PrincipalRequest {
    Ping,
}

impl Into<ZmqMessage> for ClientResponseMessage {
    fn into(self) -> ZmqMessage {
        let s = match self {
            Self::InvalidMessageType => "InvalidRequest: Unrecognised message type".to_string(),
            Self::Pong => "PONG".to_string(),
            Self::Success => "SUCCESS".to_string(),
            Self::Heartbeat(pub_id) => format!("HEARTBEAT|{}", pub_id)
        };
        ZmqMessage::from(s)
    }
}

fn parse_zmq_str(s: &str) -> (&str, Vec<String>) {
    let parsed_s: Vec<&str> = s.split("|").collect();
    let msg_type = parsed_s[0];
    let args: Vec<String> = parsed_s[1..]
        .iter()
        .map(|x| x.to_string())
        .collect();
    (msg_type, args)
}

#[async_trait]
impl BaseClientRequestMessage for AgentRequest {
    fn from_zmq_str(s: &str) -> Result<AgentRequest, ClientConversionError> {
        let (msg_type, args) = parse_zmq_str(s);
        match msg_type {
            // "GET_TASKS" => Ok(Self::GetTasks),
            "PING" => Ok(Self::Ping),
            "RECONNECT" => Ok(Self::Reconnect),
            _ => Err(ClientConversionError::new(format!("Unrecognised server message: {}", msg_type)))
        }
    }
}
impl TryFrom<ZmqMessage> for AgentRequest {
    type Error = ClientConversionError;
    fn try_from(value: ZmqMessage) -> Result<Self, Self::Error> {
        let zmq_msg_s = String::try_from(value).expect(
            "Unable to convert ZMQ Client message to String"
        );
        Self::from_zmq_str(&zmq_msg_s)
    }
}

//////////////////////////////////

// principal

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
    use super::{parse_zmq_str, AgentRequest, BaseClientRequestMessage, PrincipalRequest};

    #[test]
    fn test_agent_req_from_zmq_str(){
        let req_types = ["PING"];
        for rt in req_types {
            AgentRequest::from_zmq_str(rt).expect(
                &format!("Failed to create AgentRequest from {}", rt)
            );
        }
    }

    #[test]
    fn test_agent_req_from_zmq_str_invalid(){
        let rt = "IOASNDONTOTALLYFAKEASDKOADOAD";
        assert!(AgentRequest::from_zmq_str(rt).is_err());
    }

    #[test]
    fn test_principal_req_from_zmq_str(){
        let req_types = ["PING"];
        for rt in req_types {
            PrincipalRequest::from_zmq_str(rt).expect(
                &format!("Failed to create AgentRequest from {}", rt)
            );
        }
    }

    #[test]
    fn test_principal_req_from_zmq_str_invalid(){
        let rt = "IOASNDONTOTALLYFAKEASDKOADOAD";
        assert!(AgentRequest::from_zmq_str(rt).is_err());
    }

    #[test]
    fn test_parse_zmq_str(){
        assert!(parse_zmq_str("ECHO|THIS|THING") == ("ECHO", vec!["THIS".to_string(), "THING".to_string()]));
    }
}