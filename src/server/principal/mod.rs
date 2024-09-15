use crate::{
    db::{get_connection, models::NewScheduledTask},
    models::{AgentMeta, AgentPriorityQueue, Task, ZMQArgs},
};
use async_trait::async_trait;
use chrono::Utc;
use diesel::SqliteConnection;
use std::{collections::HashMap, sync::Arc};
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

// TODO: make an extension of AgentAPI
#[derive(Debug)]
pub enum PrincipalAPI {
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
    ///     agent_id, max_tasks
    RegisterAgent(String, usize),
}

impl TryFrom<ZmqMessage> for PrincipalAPI {
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
                Some(agent_id) => match args.next() {
                    Some(max_tasks) => {
                        let max_tasks = if let Ok(v) = max_tasks.parse::<usize>() {
                            v
                        } else {
                            return Err(RepReqError::ParseError("Arg MAX_TASKS is not a valid integer".to_string()))
                        };
                        Ok(Self::RegisterAgent(agent_id, max_tasks))
                    },
                    None => Err(RepReqError::ParseError("Missing arg MAX_TASKS".to_string()))
                },
                None => Err(RepReqError::ParseError("Missing arg AGENT_ID".to_string())),
            },
            _ => Err(RepReqError::new(
                1,
                format!("Unrecognised message type: {}", msg_type),
            )),
        }
    }
}

impl Into<ZmqMessage> for PrincipalAPI {
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
            Self::RegisterAgent(agent_id, max_tasks) => format!("REGISTERAGENT|{agent_id}|{max_tasks}"),
        };
        ZmqMessage::from(zmq_s)
    }
}

pub struct PrincipalServer {
    instance_id: String,
    db_cnxn: Arc<Mutex<SqliteConnection>>,
    live_agents: AgentPriorityQueue,
}

impl PrincipalServer {
    pub fn new(
        db_cnxn: Arc<Mutex<SqliteConnection>>, 
        instance_id: String,
        live_agents: Option<AgentPriorityQueue>,
    ) -> Self {
        let live_agents = live_agents.unwrap_or(AgentPriorityQueue::new());
        Self {
            db_cnxn,
            live_agents,
            instance_id,
        }
    }

    /// Registers the agent with the principal server. If it exists
    /// already then it simply updates with the latest timestamp
    async fn register_agent(&mut self, agent_id: &String, max_tasks: usize) -> (ClientResponseMessage, usize) {
        let now = Utc::now().timestamp_micros();
        let update_result = self.live_agents.update_timestamp(agent_id, now).await;
        match update_result {
            Ok(_) => (),
            Err(_e) => {
                // agent not registered before so add new
                let agent_meta = AgentMeta::new(agent_id.clone(), max_tasks, now);
                self.live_agents.push(agent_meta).await
            }
        };
        (ClientResponseMessage::Success, 0)
    }

}

#[async_trait]
impl Server<PrincipalAPI> for PrincipalServer {
    async fn handle_client_message(
        &mut self,
        cli_msg: PrincipalAPI,
    ) -> (ClientResponseMessage, usize) {
        match cli_msg {
            PrincipalAPI::Ping => (ClientResponseMessage::Pong, 0),
            PrincipalAPI::CreateTask(new_task) => {
                let mut db_cnxn = self.db_cnxn.lock().await;
                handle_create_task(&mut db_cnxn, new_task)
            }
            PrincipalAPI::ListTasks => {
                let mut db_cnxn = self.db_cnxn.lock().await;
                handle_list_tasks(&mut db_cnxn)
            }
            PrincipalAPI::DeleteTask(task_id) => {
                let mut db_cnxn = self.db_cnxn.lock().await;
                handle_delete_task(&mut db_cnxn, task_id)
            }
            PrincipalAPI::RunTask(agent_id, task) => {
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
            PrincipalAPI::RegisterAgent(agent_id, max_tasks) => self.register_agent(&agent_id, max_tasks).await,
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
            "REGISTERAGENT|8999|2",
        ];
        for zmq_s in all_happies {
            let zmq_msg = ZmqMessage::from(zmq_s);
            let res = PrincipalAPI::try_from(zmq_msg);
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
            ("REGISTERAGENT|8999|3", ClientResponseMessage::Success, 0),
        ];
        let mut server = PrincipalServer::new(get_db(), "fake_ins".to_string(), None);
        for (zmq_s, response, exp_exit_code) in test_params {
            let zmq_msg = ZmqMessage::from(zmq_s);
            let ar = PrincipalAPI::try_from(zmq_msg)
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
        let mut server = PrincipalServer::new(get_db(), "fake_ins".to_string(), None);
        let agent_id = String::from("fake_id");
        let (resp, exit_code) = server.register_agent(&agent_id, 3).await;
        {
            server.live_agents.pop().await.unwrap();
        }
        assert!(resp == ClientResponseMessage::Success);
        assert!(exit_code == 0)
    }

    #[tokio::test]
    async fn test_register_agent_already_exists() {
        let mut server = PrincipalServer::new(get_db(), "fake_ins".to_string(), None);
        let agent_id = String::from("fake_id");
        let max_tasks = 3;
        server.register_agent(&agent_id, max_tasks).await;
        let old_timestamp = {
            server
                .live_agents
                .pop()
                .await
                .unwrap()
                .get_last_ping_ts()
        };
        sleep(Duration::from_micros(10));
        let (resp, exit_code) = server.register_agent(&agent_id, 3).await;
        let new_timestamp = {
            server
                .live_agents
                .pop()
                .await
                .unwrap()
                .get_last_ping_ts()
        };
        assert!(new_timestamp > old_timestamp);
        assert!(resp == ClientResponseMessage::Success);
        assert!(exit_code == 0)
    }

}
