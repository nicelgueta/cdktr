use crate::{db::{get_connection, models::NewScheduledTask}, models::{Task, ZMQArgs}};
use async_trait::async_trait;
use diesel::SqliteConnection;
use std::sync::Arc;
use tokio::sync::Mutex;
use zeromq::{PubSocket, Socket, ZmqMessage};

mod api;

use api::{
    // zmq msgs
    create_new_task_payload,
    delete_task_payload,
    create_run_task_payload,

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

    // /// Runs task on a specific agent
    // /// Args:
    // ///     agent_id, task
    RunTask(String, Task)
}

pub struct PrincipalServer {
    publisher: Arc<Mutex<PubSocket>>,
    db_cnxn: SqliteConnection,
}

impl PrincipalServer {
    pub fn new(publisher: Arc<Mutex<PubSocket>>, database_url: Option<String>) -> Self {
        let db_cnxn = get_connection(database_url.as_deref());
        Self {
            publisher,
            db_cnxn,
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
            PrincipalRequest::RunTask(agent_id, task) => handle_run_task(agent_id, task).await
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
            return Err(RepReqError::ParseError(
                format!("Empty message")
            ))
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
            },
            _ => Err(RepReqError::new(
                1,
                format!("Unrecognised message type: {}", msg_type),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
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
        ];
        let fake_publisher = Arc::new(Mutex::new(PubSocket::new()));
        let mut server = PrincipalServer::new(fake_publisher, None);
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
}
