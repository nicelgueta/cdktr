use std::time::Duration;

/// API module to provide all of the principal message handling
/// utilities
///
use crate::{
    db::models::{NewScheduledTask, ScheduledTask},
    macros::args_to_model,
    models::{Task, ZMQArgs},
    server::{
        agent::AgentRequest,
        models::{ClientResponseMessage, RepReqError},
    },
    zmq_helpers::{get_agent_tcp_uri, get_req_timeout, get_zmq_req},
};
use diesel::prelude::*;

use diesel::RunQueryDsl;
use zeromq::{SocketRecv, SocketSend};

pub fn create_new_task_payload(args: ZMQArgs) -> Result<NewScheduledTask, RepReqError> {
    // TODO: make obvious that we only care about the first arg and that it's JSON
    args_to_model!(args, NewScheduledTask)
}

pub fn create_run_task_payload(mut args: ZMQArgs) -> Result<(String, Task), RepReqError> {
    let agent_id = if let Some(id) = args.next() {
        id
    } else {
        return Err(RepReqError::ParseError(
            "Missing first argument `agent_id`".to_string(),
        ));
    };
    match Task::try_from(args) {
        Ok(task) => Ok((agent_id, task)),
        Err(e) => Err(RepReqError::ParseError(format!(
            "Invalid task definition. Error: {}",
            e.to_string()
        ))),
    }
}

pub fn delete_task_payload(mut args: ZMQArgs) -> Result<i32, RepReqError> {
    if let Some(v) = args.next() {
        match v.parse() {
            Ok(v) => Ok(v),
            Err(e) => Err(RepReqError::new(
                1,
                format!(
                    "Unable to create integer from value '{}'. Error: {}",
                    &v,
                    e.to_string()
                ),
            )),
        }
    } else {
        Err(RepReqError::new(
            1,
            "No payload found for DELETETASK command. Requires TASK_ID".to_string(),
        ))
    }
}

pub fn agent_cap_reached(mut args: ZMQArgs) -> Result<(String, bool), RepReqError> {
    let agent_id = if let Some(agent_id) = args.next() {
        agent_id
    } else {
        return Err(RepReqError::ParseError("Missing AGENT_ID".to_string()));
    };
    if let Some(flag) = args.next() {
        let r_as_bool = flag.parse();
        if let Ok(b) = r_as_bool {
            Ok((agent_id, b))
        } else {
            Err(RepReqError::ParseError(
                "Could not parse arg REACHED as bool".to_string(),
            ))
        }
    } else {
        Err(RepReqError::ParseError("Missing bool: REACHED".to_string()))
    }
}

pub fn handle_create_task(
    db_cnxn: &mut SqliteConnection,
    scheduled_task: NewScheduledTask,
) -> (ClientResponseMessage, usize) {
    use crate::db::schema::schedules;
    let result = diesel::insert_into(schedules::table)
        .values(&scheduled_task)
        .execute(db_cnxn);
    match result {
        Ok(_v) => (ClientResponseMessage::Success, 0),
        Err(e) => (ClientResponseMessage::ServerError(e.to_string()), 0),
    }
}

pub fn handle_list_tasks(db_cnxn: &mut SqliteConnection) -> (ClientResponseMessage, usize) {
    use crate::db::schema::schedules::dsl::*;
    let results: Result<Vec<ScheduledTask>, diesel::result::Error> =
        schedules.select(ScheduledTask::as_select()).load(db_cnxn);
    match results {
        Ok(res) => match serde_json::to_string(&res) {
            Ok(json_str) => (ClientResponseMessage::SuccessWithPayload(json_str), 0),
            Err(e) => (
                ClientResponseMessage::ServerError(format!(
                    "Failed to convert data to JSON string. Got error: {}",
                    e.to_string()
                )),
                0,
            ),
        },
        Err(e) => (
            ClientResponseMessage::ServerError(format!(
                "Failed to query data from database. Got error: {}",
                e.to_string()
            )),
            0,
        ),
    }
}

pub fn handle_delete_task(
    db_cnxn: &mut SqliteConnection,
    task_id: i32,
) -> (ClientResponseMessage, usize) {
    use crate::db::schema::schedules::dsl::*;
    let result = diesel::delete(schedules.filter(id.eq(task_id))).execute(db_cnxn);
    match result {
        Ok(num_affected) => {
            if num_affected >= 1 {
                (ClientResponseMessage::Success, 0)
            } else {
                (
                    ClientResponseMessage::Unprocessable(format!(
                        "No records found for task_id {task_id}"
                    )),
                    0,
                )
            }
        }
        Err(e) => {
            let msg = format!(
                "Failed to convert data to JSON string. Got error: {}",
                e.to_string()
            );
            (ClientResponseMessage::ServerError(msg), 0)
        }
    }
}

pub async fn handle_run_task(agent_id: String, task: Task) -> (ClientResponseMessage, usize) {
    let req_r = get_req_timeout(&agent_id, Duration::from_micros(500)).await;
    let mut req = if let Ok(req) = req_r {
        req
    } else {
        return (ClientResponseMessage::Unprocessable(format!("Agent {} is not reachable", &agent_id)), 0)
    };
    req.send(AgentRequest::Run(task).into()).await.unwrap();
    let response = req.recv().await;
    match response {
        Ok(msg) => (ClientResponseMessage::from(msg), 0),
        Err(e) => (
            ClientResponseMessage::ServerError(format!(
                "Failed to process ZMQ message. Got: {}",
                e.to_string()
            )),
            0,
        ),
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::zmq_helpers::get_zmq_rep;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    use zeromq::ZmqMessage;
    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

    fn setup_db() -> SqliteConnection {
        let mut cnxn = SqliteConnection::establish(":memory:").unwrap();
        cnxn.run_pending_migrations(MIGRATIONS).unwrap();
        cnxn
    }

    #[test]
    fn test_create_task_payload_happy() {
        let args = vec![
            r#"{"task_name": "echo hello","task_type": "PROCESS","command": "echo","args": "hello","cron": "0 3 * * * *","next_run_timestamp": 1720313744}"#.to_string()
        ];
        assert!(create_new_task_payload(ZMQArgs::from(args)).is_ok())
    }

    #[test]
    fn test_create_task_payload_invalid_json() {
        let args = vec![r#"{"task_name": "#.to_string()];
        assert!(create_new_task_payload(ZMQArgs::from(args)).is_err())
    }

    #[test]
    fn test_create_task_payload_valid_json_but_not_task() {
        let args = vec![r#"{"task_name": "missing all other props"}"#.to_string()];
        assert!(create_new_task_payload(ZMQArgs::from(args)).is_err())
    }

    #[test]
    fn test_delete_task_happy() {
        let args = vec!["1".to_string()];
        assert!(delete_task_payload(ZMQArgs::from(args)).is_ok())
    }

    #[test]
    fn test_delete_task_invalid() {
        let args = vec!["not_an_int".to_string()];
        assert!(delete_task_payload(ZMQArgs::from(args)).is_err())
    }

    #[test]
    fn test_handle_create_task_happy() {
        let mut db_cnxn = setup_db();
        let task = NewScheduledTask {
            task_name: "echo hello".to_string(),
            task_type: "PROCESS".to_string(),
            command: "echo".to_string(),
            args: "hello".to_string(),
            cron: "0 3 * * * *".to_string(),
            next_run_timestamp: 1720313744,
        };
        assert_eq!(
            handle_create_task(&mut db_cnxn, task),
            (ClientResponseMessage::Success, 0)
        )
    }

    #[test]
    fn test_handle_list_tasks_empty_db() {
        let mut db_cnxn = setup_db();
        assert_eq!(
            handle_list_tasks(&mut db_cnxn),
            (
                ClientResponseMessage::SuccessWithPayload("[]".to_string()),
                0
            )
        )
    }

    #[test]
    fn test_handle_list_tasks_1_in_db() {
        let mut db_cnxn = setup_db();
        let task = NewScheduledTask {
            task_name: "echo hello".to_string(),
            task_type: "PROCESS".to_string(),
            command: "echo".to_string(),
            args: "hello".to_string(),
            cron: "0 3 * * * *".to_string(),
            next_run_timestamp: 1720313744,
        };
        handle_create_task(&mut db_cnxn, task);

        let (resp, exit_code) = handle_list_tasks(&mut db_cnxn);
        assert_eq!(exit_code, 0);
        match resp {
            ClientResponseMessage::SuccessWithPayload(json_str) => {
                let tasks: Vec<ScheduledTask> = serde_json::from_str(&json_str).unwrap();
                assert_eq!(tasks.len(), 1);
            }
            _ => panic!("Expected SuccessWithPayload but got {:?}", resp),
        }
    }

    #[test]
    fn test_handle_delete_task_happy() {
        let mut db_cnxn = setup_db();
        let task = NewScheduledTask {
            task_name: "echo hello".to_string(),
            task_type: "PROCESS".to_string(),
            command: "echo".to_string(),
            args: "hello".to_string(),
            cron: "0 3 * * * *".to_string(),
            next_run_timestamp: 1720313744,
        };
        handle_create_task(&mut db_cnxn, task);
        assert_eq!(
            handle_delete_task(&mut db_cnxn, 1),
            (ClientResponseMessage::Success, 0)
        )
    }

    #[test]
    fn test_handle_delete_task_not_found() {
        let mut db_cnxn = setup_db();
        let task = NewScheduledTask {
            task_name: "echo hello".to_string(),
            task_type: "PROCESS".to_string(),
            command: "echo".to_string(),
            args: "hello".to_string(),
            cron: "0 3 * * * *".to_string(),
            next_run_timestamp: 1720313744,
        };
        handle_create_task(&mut db_cnxn, task);
        assert_eq!(
            handle_delete_task(&mut db_cnxn, 2),
            (
                ClientResponseMessage::Unprocessable("No records found for task_id 2".to_string()),
                0
            )
        )
    }

    #[test]
    fn test_get_agent_tcp_uri() {
        let agent_id = "1234".to_string();
        assert_eq!(get_agent_tcp_uri(&agent_id), "tcp://0.0.0.0:1234")
    }

    #[tokio::test]
    async fn test_handle_run_task_happy() {
        // create ZMQ server to respond
        let join_h = tokio::spawn(async move {
            let mut rep = get_zmq_rep("tcp://0.0.0.0:32145").await;
            let msg_recv_s = String::try_from(rep.recv().await.unwrap()).unwrap();
            assert_eq!(&msg_recv_s, "RUN|PROCESS|echo|hello");
            rep.send(ZmqMessage::from("OK")).await
        });

        let agent_id = "32145".to_string();
        let task = Task::try_from(ZMQArgs::from(vec![
            "PROCESS".to_string(),
            "echo".to_string(),
            "hello".to_string(),
        ]))
        .expect("Failed to create task for test");
        let (resp, exit_code) = handle_run_task(agent_id, task).await;
        assert_eq!(exit_code, 0);
        match resp {
            ClientResponseMessage::Success => (),
            _ => panic!("Expected Success but got {:?}", resp),
        };
        let _res = join_h.await.unwrap().unwrap();
    }
}
