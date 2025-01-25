use async_trait::async_trait;
use chrono::Utc;
use diesel::SqliteConnection;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    api::PrincipalAPI,
    models::{AgentMeta, Task},
    utils::{
        data_structures::{AgentPriorityQueue, AsyncQueue},
        split_instance_id,
    },
};

use super::{models::ClientResponseMessage, traits::Server};

mod helpers;

pub struct PrincipalServer {
    instance_id: String,
    db_cnxn: Arc<Mutex<SqliteConnection>>,
    live_agents: AgentPriorityQueue,
    task_queue: AsyncQueue<Task>,
}

impl PrincipalServer {
    pub fn new(db_cnxn: Arc<Mutex<SqliteConnection>>, instance_id: String) -> Self {
        Self {
            db_cnxn,
            instance_id,
            live_agents: AgentPriorityQueue::new(),
            task_queue: AsyncQueue::new(),
        }
    }

    /// Registers the agent with the principal server. If it exists
    /// already then it simply updates with the latest timestamp
    async fn register_agent(&mut self, agent_id: &String) -> (ClientResponseMessage, usize) {
        let now = Utc::now().timestamp_micros();
        let update_result = self.live_agents.update_timestamp(agent_id, now).await;
        match update_result {
            Ok(_) => (),
            Err(_e) => {
                // agent not registered before so add new
                let (a_host, a_port) = split_instance_id(agent_id);
                let agent_meta = AgentMeta::new(a_host, a_port, now);
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
                helpers::handle_create_task(&mut db_cnxn, new_task)
            }
            PrincipalAPI::ListTasks => {
                let mut db_cnxn = self.db_cnxn.lock().await;
                helpers::handle_list_tasks(&mut db_cnxn)
            }
            PrincipalAPI::DeleteTask(task_id) => {
                let mut db_cnxn = self.db_cnxn.lock().await;
                helpers::handle_delete_task(&mut db_cnxn, task_id)
            }
            PrincipalAPI::AddTask(task) => {
                helpers::handle_add_task(task, &mut self.task_queue).await
            }
            PrincipalAPI::RegisterAgent(agent_id) => self.register_agent(&agent_id).await,
            PrincipalAPI::AgentTaskStatusUpdate(agent_id, task_id, status) => {
                helpers::handle_agent_task_status_update(
                    self.live_agents.clone(),
                    &task_id,
                    &status,
                )
                .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread::sleep, time::Duration};

    use zeromq::{Socket, SocketRecv, SocketSend, ZmqMessage};

    use super::*;
    use crate::db::get_connection;

    fn get_db() -> Arc<Mutex<SqliteConnection>> {
        let db = get_connection(None);
        Arc::new(Mutex::new(db))
    }

    #[test]
    fn test_principal_request_from_zmq_str_all_happy() {
        let all_happies = vec![
            "PING",
            "LISTTASKS",
            "CREATETASK|echo hello|PROCESS|echo|hello|0 3 * * * *|1720313744",
            "DELETETASK|1",
            "ADDTASK|PROCESS|echo|hello",
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
        // simulate receipt of a success message from a server
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
                "CREATETASK|echo hello|PROCESS|echo|hello|0 3 * * * *|1720313744",
                ClientResponseMessage::Success,
                0,
            ),
            ("DELETETASK|1", ClientResponseMessage::Success, 0),
            (
                "ADDTASK|PROCESS|echo|hello",
                ClientResponseMessage::Success,
                0,
            ),
            (
                "REGISTERAGENT|localhost-8999|3",
                ClientResponseMessage::Success,
                0,
            ),
        ];

        let mut server = PrincipalServer::new(get_db(), "fake_ins".to_string());
        for (zmq_s, response, exp_exit_code) in test_params {
            println!("Testing {zmq_s}");
            let zmq_msg = ZmqMessage::from(zmq_s);
            let ar = PrincipalAPI::try_from(zmq_msg)
                .expect("Should be able to unwrap the agent from ZMQ command");
            let (resp, exit_code) = server.handle_client_message(ar).await;
            dbg!(&resp);
            assert_eq!(response, resp);
            assert_eq!(exit_code, exp_exit_code);
        }
    }

    #[tokio::test]
    async fn test_register_agent_new() {
        let mut server = PrincipalServer::new(get_db(), "fake_ins".to_string());
        let agent_id = String::from("localhost-4567");
        let (resp, exit_code) = server.register_agent(&agent_id).await;
        {
            server.live_agents.pop().await.unwrap();
        }
        assert!(resp == ClientResponseMessage::Success);
        assert!(exit_code == 0)
    }

    #[tokio::test]
    async fn test_register_agent_already_exists() {
        let mut server = PrincipalServer::new(get_db(), "fake_ins".to_string());
        let agent_id = String::from("localhost-4567");
        server.register_agent(&agent_id).await;
        let old_timestamp = { server.live_agents.pop().await.unwrap().get_last_ping_ts() };
        sleep(Duration::from_micros(10));
        let (resp, exit_code) = server.register_agent(&agent_id).await;
        let new_timestamp = { server.live_agents.pop().await.unwrap().get_last_ping_ts() };
        assert!(new_timestamp > old_timestamp);
        assert!(resp == ClientResponseMessage::Success);
        assert!(exit_code == 0)
    }
}
