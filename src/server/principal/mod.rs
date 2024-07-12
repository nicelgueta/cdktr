use crate::{
    db::{get_connection, models::NewScheduledTask},
    models::{Task, ZMQArgs},
};
use async_trait::async_trait;
use chrono::Utc;
use diesel::SqliteConnection;
use std::collections::HashMap;
use zeromq::ZmqMessage;
mod api;

use api::{
    // zmq msgs
    create_new_task_payload,
    create_run_task_payload,

    delete_task_payload,
    // client handling
    handle_create_task,
    handle_delete_task,
    handle_list_tasks,
    handle_run_task,
};

use super::{
    models::{ClientResponseMessage, RepReqError},
    traits::Server,
};

// TODO: make an extension of AgentRequest
pub enum PrincipalRequest {
    /// Check server is online
    Ping,
    /// Creates a new scheudled task in the principal database
    CreateTask(NewScheduledTask),
    /// Lists all scheduled tasks currently stored in the database
    ListTasks,
    /// Deletes a specific scheduled task in the database by its id
    DeleteTask(i32),
    /// Runs task on a specific agent
    /// Args:
    ///     agent_id, task
    RunTask(String, Task),
    /// Allows an agent to register itself with the principal
    /// so that the principal can set a heartbeat for it. If the agent
    /// is already registered then this behaves in a similar way to
    /// a PING/PONG
    /// Args:
    ///     agent_id
    RegisterAgent(String),
    /// Allows agents to inform the principal of when they have reached
    /// their concurrent thread capacity. Sending a negative bool
    /// unsets this flag
    /// Args:
    ///     agent_id, flag
    AgentCapacityReached(String, bool),
}

#[derive(PartialEq, Debug)]
struct AgentConfig {
    max_threads_reached: bool,
    last_ping_timestamp: i64,
}
impl AgentConfig {
    fn new(max_threads_reached: bool, last_ping_timestamp: i64) -> Self {
        Self {
            max_threads_reached,
            last_ping_timestamp,
        }
    }
    fn update_timestamp(&mut self, new_ts: i64) {
        self.last_ping_timestamp = new_ts
    }
    fn set_max_threads_reached(&mut self, reached: bool) {
        self.max_threads_reached = reached
    }
}
pub struct PrincipalServer {
    db_cnxn: SqliteConnection,
    live_agents: HashMap<String, AgentConfig>,
}

impl PrincipalServer {
    pub fn new(database_url: Option<String>) -> Self {
        let db_cnxn = get_connection(database_url.as_deref());
        Self {
            db_cnxn,
            live_agents: HashMap::new(),
        }
    }
    /// Registers the agent with the principal server. If it exists
    /// already then it simply updates with the latest timestamp
    fn register_agent(&mut self, agent_id: &String) -> (ClientResponseMessage, usize) {
        let now = Utc::now().timestamp_micros();
        match self.live_agents.get_mut(agent_id) {
            Some(agent_config) => {
                agent_config.update_timestamp(now);
            }
            None => {
                let agent_config = AgentConfig::new(false, now);
                self.live_agents.insert(agent_id.clone(), agent_config);
            }
        };
        (ClientResponseMessage::Success, 0)
    }

    /// sets the flag on the agent config to note it's currently at capacity
    fn set_agent_at_capacity(
        &mut self,
        agent_id: &String,
        reached: bool,
    ) -> (ClientResponseMessage, usize) {
        let now = Utc::now().timestamp_micros();
        match self.live_agents.get_mut(agent_id) {
            Some(agent_config) => {
                agent_config.set_max_threads_reached(reached);
                agent_config.update_timestamp(now);
                (ClientResponseMessage::Success, 0)
            }
            None => (
                ClientResponseMessage::ClientError(String::from(
                    "Agent has not been registered - cannot set max threads reached",
                )),
                0,
            ),
        }
    }
}

#[async_trait]
impl Server<PrincipalRequest> for PrincipalServer {
    async fn handle_client_message(
        &mut self,
        cli_msg: PrincipalRequest,
    ) -> (ClientResponseMessage, usize) {
        match cli_msg {
            PrincipalRequest::Ping => (ClientResponseMessage::Pong, 0),
            PrincipalRequest::CreateTask(new_task) => {
                handle_create_task(&mut self.db_cnxn, new_task)
            }
            PrincipalRequest::ListTasks => handle_list_tasks(&mut self.db_cnxn),
            PrincipalRequest::DeleteTask(task_id) => handle_delete_task(&mut self.db_cnxn, task_id),
            PrincipalRequest::RunTask(agent_id, task) => handle_run_task(agent_id, task).await,
            PrincipalRequest::RegisterAgent(agent_id) => self.register_agent(&agent_id),
            PrincipalRequest::AgentCapacityReached(agent_id, reached) => {
                self.set_agent_at_capacity(&agent_id, reached)
            }
        }
    }
}

impl TryFrom<ZmqMessage> for PrincipalRequest {
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
            "CREATETASK" => Ok(Self::CreateTask(create_new_task_payload(args)?)),
            "LISTTASKS" => Ok(Self::ListTasks),
            "DELETETASK" => Ok(Self::DeleteTask(delete_task_payload(args)?)),
            "AGENTRUN" => {
                let (agent_id, task) = create_run_task_payload(args)?;
                Ok(Self::RunTask(agent_id, task))
            }
            _ => Err(RepReqError::new(
                1,
                format!("Unrecognised message type: {}", msg_type),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{thread::sleep, time::Duration};

    use super::*;

    #[test]
    fn test_principal_request_from_zmq_str_all_happy() {
        let all_happies = vec![
            "PING",
            "LISTTASKS",
            r#"CREATETASK|{"task_name": "echo hello","task_type": "PROCESS","command": "echo","args": "hello","cron": "0 3 * * * *","next_run_timestamp": 1720313744}"#,
            "DELETETASK|1",
        ];
        for zmq_s in all_happies {
            let zmq_msg = ZmqMessage::from(zmq_s);
            let res = PrincipalRequest::try_from(zmq_msg);
            assert!(res.is_ok())
        }
    }

    #[tokio::test]
    async fn test_handle_cli_message_all_happy() {
        let test_params = vec![
            ("PING", ClientResponseMessage::Pong, 0),
            (
                "LISTTASKS",
                ClientResponseMessage::SuccessWithPayload("[]".to_string()),
                0,
            ),
            (
                r#"CREATETASK|{"task_name": "echo hello","task_type": "PROCESS","command": "echo","args": "hello","cron": "0 3 * * * *","next_run_timestamp": 1720313744}"#,
                ClientResponseMessage::Success,
                0,
            ),
            ("DELETETASK|1", ClientResponseMessage::Success, 0),
        ];
        let mut server = PrincipalServer::new(None);
        for (zmq_s, response, exp_exit_code) in test_params {
            let zmq_msg = ZmqMessage::from(zmq_s);
            let ar = PrincipalRequest::try_from(zmq_msg)
                .expect("Should be able to unwrap the agent from ZMQ command");
            let (resp, exit_code) = server.handle_client_message(ar).await;
            println!("Testing {zmq_s}");
            assert_eq!(response, resp);
            assert_eq!(exit_code, exp_exit_code);
        }
    }

    #[test]
    fn test_register_agent_new() {
        let mut server = PrincipalServer::new(None);
        let agent_id = String::from("fake_id");
        let (resp, exit_code) = server.register_agent(&agent_id);
        server.live_agents.get(&agent_id).unwrap();
        assert!(resp == ClientResponseMessage::Success);
        assert!(exit_code == 0)
    }

    #[test]
    fn test_register_agent_already_exists() {
        let mut server = PrincipalServer::new(None);
        let agent_id = String::from("fake_id");
        server.register_agent(&agent_id);
        let old_timestamp = server
            .live_agents
            .get(&agent_id)
            .unwrap()
            .last_ping_timestamp;
        sleep(Duration::from_micros(10));
        let (resp, exit_code) = server.register_agent(&agent_id);
        let new_timestamp = server
            .live_agents
            .get(&agent_id)
            .unwrap()
            .last_ping_timestamp;
        assert!(new_timestamp > old_timestamp);
        assert!(resp == ClientResponseMessage::Success);
        assert!(exit_code == 0)
    }

    #[test]
    fn test_set_agent_at_capacity_happy() {
        let mut server = PrincipalServer::new(None);
        let agent_id = String::from("fake_id");
        server.register_agent(&agent_id);

        let (resp, exit_code) = server.set_agent_at_capacity(&agent_id, true);
        assert!(
            server
                .live_agents
                .get(&agent_id)
                .unwrap()
                .max_threads_reached
                == true
        );
        assert!(resp == ClientResponseMessage::Success);
        assert!(exit_code == 0)
    }

    #[test]
    fn test_set_agent_at_capacity_unregistered() {
        let mut server = PrincipalServer::new(None);
        let agent_id = String::from("fake_id");

        let (resp, exit_code) = server.set_agent_at_capacity(&agent_id, true);
        assert!(match resp {
            ClientResponseMessage::ClientError(_) => true,
            _ => false,
        });
        assert!(exit_code == 0)
    }
}
