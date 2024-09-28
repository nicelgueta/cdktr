pub mod agent;
pub mod models;
pub mod principal;
pub mod traits;

#[cfg(test)]
mod tests {
    use zeromq::ZmqMessage;

    use super::{agent::AgentAPI, principal::PrincipalAPI};

    #[test]
    fn test_agent_req_from_zmq_str() {
        let req_types = ["PING"];
        for rt in req_types {
            AgentAPI::try_from(ZmqMessage::from(rt))
                .expect(&format!("Failed to create AgentAPI from {}", rt));
        }
    }

    #[test]
    fn test_principal_req_from_zmq_str() {
        let req_types = ["PING"];
        for rt in req_types {
            PrincipalAPI::try_from(ZmqMessage::from(rt))
                .expect(&format!("Failed to create AgentAPI from {}", rt));
        }
    }

    #[test]
    fn test_agent_req_from_zmq_str_invalid() {
        let rt = "IOASNDONTOTALLYFAKEASDKOADOAD";
        assert!(AgentAPI::try_from(ZmqMessage::from(rt)).is_err());
    }

    #[test]
    fn test_principal_req_from_zmq_str_invalid() {
        let rt = "IOASNDONTOTALLYFAKEASDKOADOAD";
        assert!(AgentAPI::try_from(ZmqMessage::from(rt)).is_err());
    }
}
