use crate::{
    db::{get_connection, models::NewScheduledTask},
    models::{AgentConfig, Task, ZMQArgs},
};
use async_trait::async_trait;
use chrono::Utc;
use core::task;
use diesel::SqliteConnection;
use std::sync::Arc;
use std::{collections::HashMap, fmt::format};
use tokio::sync::Mutex;
use zeromq::ZmqMessage;
mod api;

use api::{
    agent_cap_reached,

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
#[derive(Debug)]
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
            "REGISTERAGENT" => match args.next() {
                Some(agent_id) => Ok(Self::RegisterAgent(agent_id)),
                None => Err(RepReqError::ParseError("Missing arg AGENT_ID".to_string())),
            },
            "AGENTCAPREACHED" => {
                let (agent_id, reached) = agent_cap_reached(args)?;
                Ok(Self::AgentCapacityReached(agent_id, reached))
            }
            _ => Err(RepReqError::new(
                1,
                format!("Unrecognised message type: {}", msg_type),
            )),
        }
    }
}

impl Into<ZmqMessage> for PrincipalRequest {
    fn into(self) -> ZmqMessage {
        let zmq_s = match self {
            Self::Ping => "PING".to_string(),
            Self::CreateTask(task) => {
                let task_json = serde_json::to_string(&task)
                    .expect("Unable to convert NewScheduledTask to JSON");
                format!("CREATETASK|{}", &task_json)
            }
            Self::RunTask(agent_id, task) => {
                let task_str: String = task.into();
                format!("AGENTRUN|{agent_id}|{task_str}")
            }
            Self::DeleteTask(task_id) => format!("DELETETASK|{task_id}"),
            Self::ListTasks => "LISTTASKS".to_string(),
            Self::RegisterAgent(agent_id) => format!("REGISTERAGENT|{agent_id}"),
            Self::AgentCapacityReached(agent_id, flag) => {
                format!("AGENTCAPREACHED|{agent_id}|{flag}")
            }
        };
        ZmqMessage::from(zmq_s)
    }
}

pub struct PrincipalServer {
    instance_id: String,
    db_cnxn: Arc<Mutex<SqliteConnection>>,
    live_agents: Arc<Mutex<HashMap<String, AgentConfig>>>,
}

impl PrincipalServer {
    pub fn new(db_cnxn: Arc<Mutex<SqliteConnection>>, instance_id: String) -> Self {
        Self {
            db_cnxn,
            live_agents: Arc::new(Mutex::new(HashMap::new())),
            instance_id,
        }
    }

    /// clones the internally created live_agents hashmap ptr for use
    /// by other tokio coroutines
    pub fn get_live_agents_ptr(&self) -> Arc<Mutex<HashMap<String, AgentConfig>>> {
        self.live_agents.clone()
    }
    /// Registers the agent with the principal server. If it exists
    /// already then it simply updates with the latest timestamp
    async fn register_agent(&mut self, agent_id: &String) -> (ClientResponseMessage, usize) {
        let now = Utc::now().timestamp_micros();
        {
            let mut agents_mut = self.live_agents.lock().await;
            match agents_mut.get_mut(agent_id) {
                Some(agent_config) => {
                    agent_config.update_timestamp(now);
                }
                None => {
                    let agent_config = AgentConfig::new(false, now);
                    agents_mut.insert(agent_id.clone(), agent_config);
                }
            };
        }
        (ClientResponseMessage::Success, 0)
    }

    /// sets the flag on the agent config to note it's currently at capacity
    async fn set_agent_at_capacity(
        &mut self,
        agent_id: &String,
        reached: bool,
    ) -> (ClientResponseMessage, usize) {
        let now = Utc::now().timestamp_micros();
        {
            let mut agents_mut = self.live_agents.lock().await;
            match agents_mut.get_mut(agent_id) {
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
                let mut db_cnxn = self.db_cnxn.lock().await;
                handle_create_task(&mut db_cnxn, new_task)
            }
            PrincipalRequest::ListTasks => {
                let mut db_cnxn = self.db_cnxn.lock().await;
                handle_list_tasks(&mut db_cnxn)
            }
            PrincipalRequest::DeleteTask(task_id) => {
                let mut db_cnxn = self.db_cnxn.lock().await;
                handle_delete_task(&mut db_cnxn, task_id)
            }
            PrincipalRequest::RunTask(agent_id, task) => {
                if agent_id == self.instance_id {
                    (
                        ClientResponseMessage::ClientError(
                            "Cannot send an AGENTRUN to a PRINCIPAL instance".to_string(),
                        ),
                        0,
                    )
                } else {
                    handle_run_task(agent_id, task).await
                }
            }
            PrincipalRequest::RegisterAgent(agent_id) => self.register_agent(&agent_id).await,
            PrincipalRequest::AgentCapacityReached(agent_id, reached) => {
                self.set_agent_at_capacity(&agent_id, reached).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread::sleep, time::Duration};

    use zeromq::{Socket, SocketRecv, SocketSend, ZmqMessage};

    use super::*;

    fn get_db() -> Arc<Mutex<SqliteConnection>> {
        let db = get_connection(None);
        Arc::new(Mutex::new(db))
    }

    #[test]
    fn test_principal_request_from_zmq_str_all_happy() {
        let all_happies = vec![
            "PING",
            "LISTTASKS",
            r#"CREATETASK|{"task_name": "echo hello","task_type": "PROCESS","command": "echo","args": "hello","cron": "0 3 * * * *","next_run_timestamp": 1720313744}"#,
            "DELETETASK|1",
            "AGENTRUN|8999|PROCESS|echo|hello",
            "REGISTERAGENT|8999",
            "AGENTCAPREACHED|8999|true",
        ];
        for zmq_s in all_happies {
            let zmq_msg = ZmqMessage::from(zmq_s);
            let res = PrincipalRequest::try_from(zmq_msg);
            dbg!(&res);
            assert!(res.is_ok())
        }
    }

    #[tokio::test]
    async fn test_handle_cli_message_all_happy() {
        // simulate receipt of a message from a client
        tokio::spawn(async {
            let uri = "tcp://0.0.0.0:8999";
            let mut rep_socket = zeromq::RepSocket::new();
            rep_socket.bind(uri).await.expect("Failed to connect");
            let _ = rep_socket.recv().await.unwrap();
            rep_socket
                .send("OK".into())
                .await
                .expect("Failed to send response")
        });
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
            (
                "AGENTRUN|8999|PROCESS|echo|hello",
                ClientResponseMessage::Success,
                0,
            ),
            ("REGISTERAGENT|8999", ClientResponseMessage::Success, 0),
            (
                "AGENTCAPREACHED|8999|true",
                ClientResponseMessage::Success,
                0,
            ),
        ];
        let mut server = PrincipalServer::new(get_db(), "fake_ins".to_string());
        for (zmq_s, response, exp_exit_code) in test_params {
            let zmq_msg = ZmqMessage::from(zmq_s);
            let ar = PrincipalRequest::try_from(zmq_msg)
                .expect("Should be able to unwrap the agent from ZMQ command");
            let (resp, exit_code) = server.handle_client_message(ar).await;
            dbg!(&resp);
            println!("Testing {zmq_s}");
            assert_eq!(response, resp);
            assert_eq!(exit_code, exp_exit_code);
        }
    }

    #[tokio::test]
    async fn test_register_agent_new() {
        let mut server = PrincipalServer::new(get_db(), "fake_ins".to_string());
        let agent_id = String::from("fake_id");
        let (resp, exit_code) = server.register_agent(&agent_id).await;
        {
            server.live_agents.lock().await.get(&agent_id).unwrap();
        }
        assert!(resp == ClientResponseMessage::Success);
        assert!(exit_code == 0)
    }

    #[tokio::test]
    async fn test_register_agent_already_exists() {
        let mut server = PrincipalServer::new(get_db(), "fake_ins".to_string());
        let agent_id = String::from("fake_id");
        server.register_agent(&agent_id).await;
        let old_timestamp = {
            server
                .live_agents
                .lock()
                .await
                .get(&agent_id)
                .unwrap()
                .get_last_ping_ts()
        };
        sleep(Duration::from_micros(10));
        let (resp, exit_code) = server.register_agent(&agent_id).await;
        let new_timestamp = {
            server
                .live_agents
                .lock()
                .await
                .get(&agent_id)
                .unwrap()
                .get_last_ping_ts()
        };
        assert!(new_timestamp > old_timestamp);
        assert!(resp == ClientResponseMessage::Success);
        assert!(exit_code == 0)
    }

    #[tokio::test]
    async fn test_set_agent_at_capacity_happy() {
        let mut server = PrincipalServer::new(get_db(), "fake_ins".to_string());
        let agent_id = String::from("fake_id");
        server.register_agent(&agent_id).await;

        let (resp, exit_code) = server.set_agent_at_capacity(&agent_id, true).await;
        assert!(
            server
                .live_agents
                .lock()
                .await
                .get(&agent_id)
                .unwrap()
                .get_max_threads_reached()
                == true
        );
        assert!(resp == ClientResponseMessage::Success);
        assert!(exit_code == 0)
    }

    #[tokio::test]
    async fn test_set_agent_at_capacity_unregistered() {
        let mut server = PrincipalServer::new(get_db(), "fake_ins".to_string());
        let agent_id = String::from("fake_id");

        let (resp, exit_code) = server.set_agent_at_capacity(&agent_id, true).await;
        assert!(match resp {
            ClientResponseMessage::ClientError(_) => true,
            _ => false,
        });
        assert!(exit_code == 0)
    }
}
