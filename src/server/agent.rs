use async_trait::async_trait;
use zeromq::ZmqMessage;
use super::{
    parse_zmq_str,
    traits::{Server, BaseClientRequestMessage},
    models::{
        ClientResponseMessage,
        ClientConversionError
    }
};
pub enum AgentRequest{

    /// Check the server is online
    Ping,

    /// Check the current publisher ID that the agent is subscribed to
    Heartbeat,

    /// Command sent to instruct the agent to reset the publisher ID
    /// and restart the instance
    Reconnect(String)
}


#[async_trait]
impl BaseClientRequestMessage for AgentRequest {
    fn from_zmq_str(s: &str) -> Result<AgentRequest, ClientConversionError> {
        let (msg_type, args) = parse_zmq_str(s);
        match msg_type {
            // "GET_TASKS" => Ok(Self::GetTasks),
            "PING" => Ok(Self::Ping),
            "RECONNECT" => {
                if args.len() == 0 {
                    Err(ClientConversionError::new(
                        "RECONNECT command requires 1 argument: publisher_id".to_string()
                    ))
                } else {
                    let pub_id = args[0].clone();
                    if pub_id.len() == 0 {
                        Err(ClientConversionError::new(
                            "RECONNECT publisher_id cannot be blank".to_string()
                        ))
                    } else {
                        Ok(Self::Reconnect(pub_id))
                    }
                }
            },
            "HEARTBEAT" => Ok(Self::Heartbeat),
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


pub struct AgentServer {
    /// ID of the publisher currently subscribed to
    publisher_id: String
}

impl AgentServer {
    pub fn new() -> Self {
    
        // start with an empty string - the first heartbeat from the principal 
        //will correct this to the new value
        Self {publisher_id: "DISCONNECTED".to_string()}
    }
}

#[async_trait]
impl Server<AgentRequest> for AgentServer {
    
    async fn handle_client_message(
        &mut self, 
        cli_msg: AgentRequest
    ) -> (ClientResponseMessage, bool) {
        match cli_msg {
            AgentRequest::Ping => (ClientResponseMessage::Pong, false),
            AgentRequest::Heartbeat => (ClientResponseMessage::Heartbeat(self.publisher_id.clone()), false),
            AgentRequest::Reconnect(pub_id) => {
                self.publisher_id = pub_id;
                (ClientResponseMessage::Success, true)
            }

        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_request_from_zmq_str_all_happy(){
        const ALL_HAPPIES: [&str; 3] = [
            "PING",
            "RECONNECT|newid",
            "HEARTBEAT"
        ];
        for zmq_s in ALL_HAPPIES {
            let res = AgentRequest::from_zmq_str(zmq_s);
            assert!(res.is_ok())
        }
    }

    #[test]
    fn test_reconnect_missing_param(){
        let zs = "RECONNECT";
        let res = AgentRequest::from_zmq_str(zs);
        assert!(res.is_err())
    }

    #[test]    
    fn test_reconnect_blank_param(){
        let zs = "RECONNECT|";
        let res = AgentRequest::from_zmq_str(zs);
        assert!(res.is_err())
    }

    #[tokio::test]
    async fn test_handle_cli_message_all_happy(){
        let test_params = [
            ("PING", ClientResponseMessage::Pong, false),
            ("RECONNECT|newid", ClientResponseMessage::Success, true),
            ("HEARTBEAT", ClientResponseMessage::Heartbeat("newid".to_string()), false),
        ];
        let mut server = AgentServer::new();
        for (
            zmq_s, response, restart_flag
        ) in test_params {
            let ar = AgentRequest::from_zmq_str(zmq_s).expect(
                "Should be able to unwrap the agent from ZMQ command"
            );
            let (resp, flag) = server.handle_client_message(
                ar
            ).await;
            assert_eq!(response, resp);
            assert_eq!(flag, restart_flag);
        }

    }

}