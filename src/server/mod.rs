use zeromq::ZmqMessage;

pub mod agent;
pub mod models;
pub mod principal;
mod traits;

pub use traits::Server;

impl Into<ZmqMessage> for models::ClientResponseMessage {
    fn into(self) -> ZmqMessage {
        let s = match self {
            Self::ClientError(payload) => format!("CLIENTERROR|{payload}"),
            Self::Pong => "PONG".to_string(),
            Self::Success => "SUCCESS".to_string(),
            Self::SuccessWithPayload(payload) => format!("SUCCESS|{payload}"),
            Self::Heartbeat(pub_id) => format!("HEARTBEAT|{pub_id}"),
            Self::ServerError(payload) => format!("SERVERERROR|{payload}"),
            Self::Unprocessable(payload) => format!("UNPROC|{payload}"),
        };
        ZmqMessage::from(s)
    }
}

fn parse_zmq_str(s: &str) -> (&str, Vec<String>) {
    let parsed_s: Vec<&str> = s.split("|").collect();
    let msg_type = parsed_s[0];
    let args: Vec<String> = parsed_s[1..].iter().map(|x| x.to_string()).collect();
    (msg_type, args)
}

#[cfg(test)]
mod tests {
    use super::{
        agent::AgentRequest, parse_zmq_str, principal::PrincipalRequest,
        traits::BaseClientRequestMessage,
    };

    #[test]
    fn test_agent_req_from_zmq_str() {
        let req_types = ["PING"];
        for rt in req_types {
            AgentRequest::from_zmq_str(rt)
                .expect(&format!("Failed to create AgentRequest from {}", rt));
        }
    }

    #[test]
    fn test_principal_req_from_zmq_str() {
        let req_types = ["PING"];
        for rt in req_types {
            PrincipalRequest::from_zmq_str(rt)
                .expect(&format!("Failed to create AgentRequest from {}", rt));
        }
    }

    #[test]
    fn test_agent_req_from_zmq_str_invalid() {
        let rt = "IOASNDONTOTALLYFAKEASDKOADOAD";
        assert!(AgentRequest::from_zmq_str(rt).is_err());
    }

    #[test]
    fn test_principal_req_from_zmq_str_invalid() {
        let rt = "IOASNDONTOTALLYFAKEASDKOADOAD";
        assert!(AgentRequest::from_zmq_str(rt).is_err());
    }

    #[test]
    fn test_parse_zmq_str() {
        assert!(
            parse_zmq_str("ECHO|THIS|THING")
                == ("ECHO", vec!["THIS".to_string(), "THING".to_string()])
        );
    }
}
