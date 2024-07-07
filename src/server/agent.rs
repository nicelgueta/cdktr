use async_trait::async_trait;
use zeromq::ZmqMessage;
use crate::models::Task;

use super::{
    parse_zmq_str,
    traits::{Server, BaseClientRequestMessage},
    models::{
        ClientResponseMessage,
        RepReqError
    },
    agent_api::create_task_run_payload
};
pub enum AgentRequest{

    /// Check the server is online
    Ping,

    /// Check the current publisher ID that the agent is subscribed to
    Heartbeat,

    /// Command sent to instruct the agent to reset the publisher ID
    /// and restart the instance
    Reconnect(String),

    /// Action to run a specific task. This is the main hook used by the 
    /// principal to send tasks for execution to the agents
    Run(Task)
}


#[async_trait]
impl BaseClientRequestMessage for AgentRequest {
    fn from_zmq_str(s: &str) -> Result<AgentRequest, RepReqError> {
        let (msg_type, args) = parse_zmq_str(s);
        match msg_type {
            // "GET_TASKS" => Ok(Self::GetTasks),
            "PING" => Ok(Self::Ping),
            "RECONNECT" => {
                if args.len() == 0 {
                    Err(RepReqError::new(1,
                        "RECONNECT command requires 1 argument: publisher_id".to_string()
                    ))
                } else {
                    let pub_id = args[0].clone();
                    if pub_id.len() == 0 {
                        Err(RepReqError::new(1,
                            "RECONNECT publisher_id cannot be blank".to_string()
                        ))
                    } else {
                        Ok(Self::Reconnect(pub_id))
                    }
                }
            },
            "HEARTBEAT" => Ok(Self::Heartbeat),
            "RUN" => Ok(Self::Run(create_task_run_payload(args)?)),
            _ => Err(RepReqError::new(1,format!("Unrecognised message type: {}", msg_type)))
        }
    }
}
impl TryFrom<ZmqMessage> for AgentRequest {
    type Error = RepReqError;
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
    ) -> (ClientResponseMessage, usize) {
        match cli_msg {
            AgentRequest::Ping => (ClientResponseMessage::Pong, 0),
            AgentRequest::Heartbeat => (ClientResponseMessage::Heartbeat(self.publisher_id.clone()), 0),
            AgentRequest::Reconnect(pub_id) => {
                self.publisher_id = pub_id;
                (ClientResponseMessage::Success, 1)
            },
            AgentRequest::Run(pl) => todo!()

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
            ("PING", ClientResponseMessage::Pong, 0),
            ("RECONNECT|newid", ClientResponseMessage::Success, 1),
            ("HEARTBEAT", ClientResponseMessage::Heartbeat("newid".to_string()), 0),
        ];
        let mut server = AgentServer::new();
        for (
            zmq_s, response, exp_exit_code
        ) in test_params {
            let ar = AgentRequest::from_zmq_str(zmq_s).expect(
                "Should be able to unwrap the agent from ZMQ command"
            );
            let (resp, exit_code) = server.handle_client_message(
                ar
            ).await;
            assert_eq!(response, resp);
            assert_eq!(exit_code, exp_exit_code);
        }

    }

}