use crate::{
    db::get_connection, models::{Task, ZMQArgs}, utils::AsyncQueue, zmq_helpers::get_zmq_req
};
use async_trait::async_trait;
use zeromq::{ZmqMessage, SocketSend};
mod api;
use super::{
    models::{ClientResponseMessage, RepReqError}, principal::PrincipalRequest, traits::Server
};

pub enum AgentRequest {
    /// Check the server is online
    Ping,

    /// Check the current publisher ID that the agent is subscribed to
    Heartbeat,


    /// Action to run a specific task. This is the main hook used by the
    /// principal to send tasks for execution to the agents
    Run(Task),
}

impl TryFrom<ZmqMessage> for AgentRequest {
    type Error = RepReqError;
    fn try_from(value: ZmqMessage) -> Result<Self, Self::Error> {
        let mut args: ZMQArgs = value.into();
        let msg_type = if let Some(token) = args.next() {
            token
        } else {
            return Err(RepReqError::ParseError(format!("Empty message")));
        };
        match msg_type.as_str() {
            // "GET_TASKS" => Ok(Self::GetTasks),
            "PING" => Ok(Self::Ping),
            "HEARTBEAT" => Ok(Self::Heartbeat),
            "RUN" => Ok(Self::Run(api::create_task_run_payload(args)?)),
            _ => Err(RepReqError::new(
                1,
                format!("Unrecognised message type: {}", msg_type),
            )),
        }
    }
}

impl Into<ZmqMessage> for AgentRequest {
    fn into(self) -> ZmqMessage {
        let zmq_s = match self {
            Self::Heartbeat => "HEARTBEAT".to_string(),
            Self::Ping => "PING".to_string(),
            Self::Run(task) => {
                let s: String = task.into();
                format!("RUN|{s}")
            }
        };
        ZmqMessage::from(zmq_s)
    }
}

pub struct AgentServer {
    /// ID of the publisher currently subscribed to
    instance_id: String,
    task_queue: AsyncQueue<Task>,
}

impl AgentServer {
    pub fn new(instance_id: String, task_queue: AsyncQueue<Task>) -> Self {
        // start with an empty string - the first heartbeat from the principal
        //will correct this to the new value
        Self {
            instance_id,
            task_queue,
        }
    }
    async fn register_with_principal(&self, principal_uri: &str) {
        let mut req = get_zmq_req(principal_uri).await;
        let msg = PrincipalRequest::RegisterAgent(self.instance_id.clone());
        req.send(msg.into()).await;

    }
}

#[async_trait]
impl Server<AgentRequest> for AgentServer {
    async fn handle_client_message(
        &mut self,
        cli_msg: AgentRequest,
    ) -> (ClientResponseMessage, usize) {
        match cli_msg {
            AgentRequest::Ping => (ClientResponseMessage::Pong, 0),
            AgentRequest::Heartbeat => (
                ClientResponseMessage::Heartbeat(self.instance_id.clone()),
                0,
            ),
            AgentRequest::Run(task) => {
                self.task_queue.put(task).await;
                (ClientResponseMessage::Success, 0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_request_from_zmq_str_all_happy() {
        const ALL_HAPPIES: [&str; 2] = ["PING", "HEARTBEAT"];
        for zmq_s in ALL_HAPPIES {
            let res = AgentRequest::try_from(ZmqMessage::from(zmq_s));
            assert!(res.is_ok())
        }
    }

    #[tokio::test]
    async fn test_handle_cli_message_all_happy() {
        let test_params = [
            ("PING", ClientResponseMessage::Pong, 0),
            (
                "HEARTBEAT",
                ClientResponseMessage::Heartbeat("newid".to_string()),
                0,
            ),
        ];
        let mut server = AgentServer::new("newid".to_string(), AsyncQueue::new());
        for (zmq_s, response, exp_exit_code) in test_params {
            let ar = AgentRequest::try_from(ZmqMessage::from(zmq_s))
                .expect("Should be able to unwrap the agent from ZMQ command");
            let (resp, exit_code) = server.handle_client_message(ar).await;
            assert_eq!(response, resp);
            assert_eq!(exit_code, exp_exit_code);
        }
    }
}