use crate::{
    models::{Task, ZMQArgs},
    utils::AsyncQueue,
    zmq_helpers::get_zmq_req,
};
use async_trait::async_trait;
use log::debug;
use zeromq::{SocketSend, ZmqMessage};
mod api;
use super::{
    models::{ClientResponseMessage, RepReqError},
    principal::PrincipalAPI,
    traits::Server,
};

pub enum AgentAPI {
    /// Check the server is online
    Ping,

    /// Check the current publisher ID that the agent is subscribed to
    Heartbeat,

    /// Action to run a specific task. This is the main hook used by the
    /// principal to send tasks for execution to the agents
    Run(Task),
}

impl TryFrom<ZmqMessage> for AgentAPI {
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

impl Into<ZmqMessage> for AgentAPI {
    fn into(self) -> ZmqMessage {
        let zmq_s = match self {
            Self::Heartbeat => "HEARTBEAT".to_string(),
            Self::Ping => "PING".to_string(),
            Self::Run(task) => {
                format!("RUN|{}", task.to_string())
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
    pub async fn register_with_principal(&self, principal_uri: &str, max_tasks: usize) {
        debug!("Registering agent with principal @ {}", &principal_uri);
        let mut req = get_zmq_req(principal_uri).await;
        let msg = PrincipalAPI::RegisterAgent(self.instance_id.clone(), max_tasks);
        req.send(msg.into())
            .await
            .expect("Got ZMQ Error attempting to connect to principal");
        debug!("Successfully registered agent with principal");
    }
}

#[async_trait]
impl Server<AgentAPI> for AgentServer {
    async fn handle_client_message(&mut self, cli_msg: AgentAPI) -> (ClientResponseMessage, usize) {
        match cli_msg {
            AgentAPI::Ping => (ClientResponseMessage::Pong, 0),
            AgentAPI::Heartbeat => (
                ClientResponseMessage::Heartbeat(self.instance_id.clone()),
                0,
            ),
            AgentAPI::Run(task) => {
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
            let res = AgentAPI::try_from(ZmqMessage::from(zmq_s));
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
            let ar = AgentAPI::try_from(ZmqMessage::from(zmq_s))
                .expect("Should be able to unwrap the agent from ZMQ command");
            let (resp, exit_code) = server.handle_client_message(ar).await;
            assert_eq!(response, resp);
            assert_eq!(exit_code, exp_exit_code);
        }
    }
}
