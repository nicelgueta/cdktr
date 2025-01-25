use crate::{
    exceptions::GenericError,
    models::{Task, ZMQArgs},
    utils::data_structures::AsyncQueue,
    zmq_helpers::{get_zmq_req, DEFAULT_TIMEOUT},
};
use async_trait::async_trait;
use log::debug;
use zeromq::{SocketSend, ZmqMessage};
mod api;
use super::{
    models::{ClientResponseMessage, RepReqError},
    principal::PrincipalAPI,
    traits::{APIMeta, Server, API},
};

pub enum AgentAPI {
    /// Check the server is online
    Ping,

    /// Action to run a specific task. This is the main hook used by the
    /// principal to send tasks for execution to the agents
    Run(Task),
}
impl From<AgentAPI> for String {
    fn from(value: AgentAPI) -> Self {
        value.to_string()
    }
}

impl API for AgentAPI {
    fn get_meta(&self) -> Vec<APIMeta> {
        const META: [(&'static str, &'static str); 2] = [
            ("PING", "Check the server is online"),
            (
                "RUN",
                "Action to run a specific task. This is the main hook used by the principal to send tasks for execution to the agents",
            ),
        ];
        META.iter()
            .map(|(action, desc)| APIMeta::new(action.to_string(), desc.to_string()))
            .collect()
    }
    fn to_string(&self) -> String {
        match self {
            Self::Ping => "PING".to_string(),
            Self::Run(task) => {
                format!("RUN|{}", task.to_string())
            }
        }
    }
}

impl TryFrom<ZmqMessage> for AgentAPI {
    type Error = RepReqError;
    fn try_from(value: ZmqMessage) -> Result<Self, Self::Error> {
        let zmq_args: ZMQArgs = value.into();
        Self::try_from(zmq_args)
    }
}

impl TryFrom<ZMQArgs> for AgentAPI {
    type Error = RepReqError;
    fn try_from(mut args: ZMQArgs) -> Result<Self, Self::Error> {
        let msg_type = if let Some(token) = args.next() {
            token
        } else {
            return Err(RepReqError::ParseError(format!("Empty message")));
        };
        match msg_type.as_str() {
            // "GET_TASKS" => Ok(Self::GetTasks),
            "PING" => Ok(Self::Ping),
            "RUN" => Ok(Self::Run(api::create_task_run_payload(args)?)),
            _ => Err(RepReqError::ParseError(format!(
                "Unrecognised message type: {}",
                msg_type
            ))),
        }
    }
}
impl TryFrom<String> for AgentAPI {
    type Error = RepReqError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        let zmq_args: ZMQArgs = s.into();
        Self::try_from(zmq_args)
    }
}
impl Into<ZmqMessage> for AgentAPI {
    fn into(self) -> ZmqMessage {
        ZmqMessage::from(self.to_string())
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
    pub async fn register_with_principal(
        &self,
        principal_uri: &str,
        max_tasks: usize,
    ) -> Result<(), GenericError> {
        debug!("Registering agent with principal @ {}", &principal_uri);
        let request = PrincipalAPI::RegisterAgent(self.instance_id.clone(), max_tasks);
        match request.send(principal_uri, DEFAULT_TIMEOUT).await {
            Ok(cli_msg) => match cli_msg {
                ClientResponseMessage::Success => {
                    debug!("Successfully registered agent with principal");
                    Ok(())
                }
                other => Err(GenericError::RuntimeError(format!(
                    "Failed to register with principal. Error: {}",
                    {
                        let m: String = other.into();
                        m
                    }
                ))),
            },
            Err(e) => Err(e),
        }
    }
}

#[async_trait]
impl Server<AgentAPI> for AgentServer {
    async fn handle_client_message(&mut self, cli_msg: AgentAPI) -> (ClientResponseMessage, usize) {
        match cli_msg {
            AgentAPI::Ping => (ClientResponseMessage::Pong, 0),
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
        const ALL_HAPPIES: [&str; 2] = ["PING", "RUN|PROCESS|echo|hello"];
        for zmq_s in ALL_HAPPIES {
            let res = AgentAPI::try_from(ZmqMessage::from(zmq_s));
            assert!(res.is_ok())
        }
    }

    #[tokio::test]
    async fn test_handle_cli_message_all_happy() {
        let test_params = [
            ("PING", ClientResponseMessage::Pong, 0),
            ("RUN|PROCESS|echo|hello", ClientResponseMessage::Success, 0),
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
