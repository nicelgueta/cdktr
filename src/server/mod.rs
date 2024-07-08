use zeromq::ZmqMessage;

pub mod agent;
pub mod models;
pub mod principal;
mod traits;

pub use traits::Server;

#[cfg(test)]
mod tests {
    use zeromq::ZmqMessage;

    use super::{
        agent::AgentRequest, principal::PrincipalRequest,
    };

    #[test]
    fn test_agent_req_from_zmq_str() {
        let req_types = ["PING"];
        for rt in req_types {
            AgentRequest::try_from(ZmqMessage::from(rt))
                .expect(&format!("Failed to create AgentRequest from {}", rt));
        }
    }

    #[test]
    fn test_principal_req_from_zmq_str() {
        let req_types = ["PING"];
        for rt in req_types {
            PrincipalRequest::try_from(ZmqMessage::from(rt))
                .expect(&format!("Failed to create AgentRequest from {}", rt));
        }
    }

    #[test]
    fn test_agent_req_from_zmq_str_invalid() {
        let rt = "IOASNDONTOTALLYFAKEASDKOADOAD";
        assert!(AgentRequest::try_from(ZmqMessage::from(rt)).is_err());
    }

    #[test]
    fn test_principal_req_from_zmq_str_invalid() {
        let rt = "IOASNDONTOTALLYFAKEASDKOADOAD";
        assert!(AgentRequest::try_from(ZmqMessage::from(rt)).is_err());
    }
}
