use async_trait::async_trait;
use diesel::SqliteConnection;
use std::sync::Arc;
use tokio::sync::Mutex;
use zeromq::{ZmqMessage, PubSocket, Socket};
use crate::db::models::NewScheduledTask;
use crate::db::{models::ScheduledTask, get_connection};

use super::principal_api::{
    // zmq msgs
    create_task_payload,
    delete_task_payload,

    // client handling
    handle_create_task,
    handle_list_tasks,
    handle_delete_task
};

use super::{
    parse_zmq_str,
    traits::{Server, BaseClientRequestMessage},
    models::{
        ClientResponseMessage,
        RepReqError
    }
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
    DeleteTask(i32)
}


pub struct PrincipalServer {
    publisher: Arc<Mutex<PubSocket>>,
    req: zeromq::ReqSocket,
    db_cnxn: SqliteConnection
}

impl PrincipalServer {
    pub fn new(publisher: Arc<Mutex<PubSocket>>, database_url: Option<String>) -> Self  {
        let req = zeromq::ReqSocket::new();
        let db_cnxn = get_connection(database_url.as_deref());
        Self { publisher, req, db_cnxn }
    }
}

#[async_trait]
impl Server<PrincipalRequest> for PrincipalServer {

    async fn handle_client_message(
        &mut self, 
        cli_msg: PrincipalRequest
    ) -> (ClientResponseMessage, bool) {
        match cli_msg {
            PrincipalRequest::Ping => (ClientResponseMessage::Pong, false),
            PrincipalRequest::CreateTask(new_task) => handle_create_task(&mut self.db_cnxn, new_task),
            PrincipalRequest::ListTasks => handle_list_tasks(&mut self.db_cnxn),
            PrincipalRequest::DeleteTask(task_id) => handle_delete_task(&mut self.db_cnxn,task_id),
        }
    }
}


#[async_trait]
impl BaseClientRequestMessage for PrincipalRequest {
    fn from_zmq_str(s: &str) -> Result<PrincipalRequest, RepReqError> {
        let (msg_type, args) = parse_zmq_str(s);
        match msg_type {
            // "GET_TASKS" => Ok(Self::GetTasks),
            "PING" => Ok(Self::Ping),
            "CREATETASK" => Ok(Self::CreateTask(create_task_payload(args)?)),
            "LISTTASKS" => Ok(Self::ListTasks),
            "DELETETASK" => Ok(Self::DeleteTask(delete_task_payload(args)?)),
            _ => Err(RepReqError::new(1, format!("Unrecognised message type: {}", msg_type)))
        }
    }
}
impl TryFrom<ZmqMessage> for PrincipalRequest {
    type Error = RepReqError;
    fn try_from(value: ZmqMessage) -> Result<Self, Self::Error> {
        let zmq_msg_s = String::try_from(value).expect(
            "Unable to convert ZMQ Client message to String"
        );
        Self::from_zmq_str(&zmq_msg_s)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_principal_request_from_zmq_str_all_happy(){
        let all_happies = vec![
            "PING",
            "LISTTASKS",
            r#"CREATETASK|{"task_name": "echo hello","task_type": "PROCESS","command": "echo","args": "hello","cron": "0 3 * * * *","next_run_timestamp": 1720313744}"#,
            "DELETETASK|1"
        ];
        for zmq_s in all_happies {
            let res = PrincipalRequest::from_zmq_str(zmq_s);
            assert!(res.is_ok())
        }
    }

    #[tokio::test]
    async fn test_handle_cli_message_all_happy(){
        let test_params = vec![
            (
                "PING", 
                ClientResponseMessage::Pong, false
            ),
            (
                "LISTTASKS", 
                ClientResponseMessage::SuccessWithPayload("[]".to_string()), false
            ),
            (
                r#"CREATETASK|{"task_name": "echo hello","task_type": "PROCESS","command": "echo","args": "hello","cron": "0 3 * * * *","next_run_timestamp": 1720313744}"#, 
                ClientResponseMessage::Success, false
            ),
        ];
        let fake_publisher = Arc::new(Mutex::new(PubSocket::new()));
        let mut server = PrincipalServer::new(fake_publisher, None);
        for (
            zmq_s, response, restart_flag
        ) in test_params {
            let ar = PrincipalRequest::from_zmq_str(zmq_s).expect(
                "Should be able to unwrap the agent from ZMQ command"
            );
            let (resp, flag) = server.handle_client_message(
                ar
            ).await;
            println!("Testing {zmq_s}");
            assert_eq!(response, resp);
            assert_eq!(flag, restart_flag);
        }

    }
}