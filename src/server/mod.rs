use zeromq::ZmqMessage;

mod traits;
mod principal_api;
pub mod models;
pub mod agent;
pub mod principal;

pub use traits::Server;

impl Into<ZmqMessage> for models::ClientResponseMessage {
    fn into(self) -> ZmqMessage {
        let s = match self {
            Self::InvalidMessageType => "InvalidRequest: Unrecognised message type".to_string(),
            Self::ClientError(payload) => format!("ClientError: {}", payload),
            Self::Pong => "PONG".to_string(),
            Self::Success => "SUCCESS".to_string(),
            Self::SuccessWithPayload(payload) => format!("SUCCESS|{}", payload),
            Self::Heartbeat(pub_id) => format!("HEARTBEAT|{}", pub_id),
            Self::ServerError(payload) => format!("ServerError: {}", payload)
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

#[cfg(test)]
mod tests {
    use super::{
        parse_zmq_str, 
        agent::AgentRequest,
        principal::PrincipalRequest,
        traits::BaseClientRequestMessage, 
    };

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
    fn test_principal_req_from_zmq_str(){
        let req_types = ["PING"];
        for rt in req_types {
            PrincipalRequest::from_zmq_str(rt).expect(
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
    fn test_principal_req_from_zmq_str_invalid(){
        let rt = "IOASNDONTOTALLYFAKEASDKOADOAD";
        assert!(AgentRequest::from_zmq_str(rt).is_err());
    }

    #[test]
    fn test_parse_zmq_str(){
        assert!(parse_zmq_str("ECHO|THIS|THING") == ("ECHO", vec!["THIS".to_string(), "THING".to_string()]));
    }
}