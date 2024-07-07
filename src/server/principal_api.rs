use diesel::prelude::*;
/// API module to provide all of the principal message handling
/// utilities
/// 
use crate::db::models::{NewScheduledTask, ScheduledTask};
use super::models::{RepReqError, ClientResponseMessage};
use diesel::RunQueryDsl;

pub fn create_task_payload(args: Vec<String>) -> Result<NewScheduledTask, RepReqError> {
    if args.len() == 0 {
        Err(RepReqError::new(
            1,
            "No payload found for CREATETASK command".to_string()
        ))
    } else {
        let parse_res: Result<NewScheduledTask, serde_json::Error> = serde_json::from_str(&args[0]);
        match parse_res{
            Ok(task) => Ok(task),
            Err(e) => Err(RepReqError::new(
                1, 
                format!("Invalid JSON for ScheduledTask. Error: {}", e.to_string())
            ))
        }
    }
}

pub fn delete_task_payload(args: Vec<String>) -> Result<i32, RepReqError> {
    if args.len() == 0 {
        Err(RepReqError::new(
            1, 
            "No payload found for DELETETASK command. Requires task ID".to_string()
        ))
    } else {
        match args[0].parse() {
            Ok(v) => Ok(v),
            Err(e) => Err(RepReqError::new(1,
                format!("Unable to create integer from value '{}'. Error: {}",&args[0], e.to_string())
            ))
        }
    }
}

pub fn handle_create_task(db_cnxn: &mut SqliteConnection, scheduled_task: NewScheduledTask) -> (ClientResponseMessage, usize) {
    use crate::db::schema::schedules;
    let result = diesel::insert_into(schedules::table)
        .values(&scheduled_task)
        .execute(db_cnxn)
    ;
    match result {
        Ok(_v) => (ClientResponseMessage::Success, 0),
        Err(e) => (ClientResponseMessage::ServerError(e.to_string()), 0)
    }

}

pub fn handle_list_tasks(db_cnxn: &mut SqliteConnection) -> (ClientResponseMessage, usize) {
    use crate::db::schema::schedules::dsl::*; 
    let results: Result<Vec<ScheduledTask>, diesel::result::Error> = schedules
        .select(ScheduledTask::as_select())
        .load(db_cnxn)
    ;
    match results {
        Ok(res) => {
            match serde_json::to_string(&res) {
                Ok(json_str) => (ClientResponseMessage::SuccessWithPayload(json_str), 0),
                Err(e) => {
                    (
                        ClientResponseMessage::ServerError(
                            format!("Failed to convert data to JSON string. Got error: {}",e.to_string())
                        ), 
                        0
                    )
                }
            }
        },
        Err(e) => (ClientResponseMessage::ServerError(
            format!("Failed to query data from database. Got error: {}",e.to_string())
        ), 0)
    }
}


pub fn handle_delete_task(db_cnxn: &mut SqliteConnection, task_id: i32) -> (ClientResponseMessage, usize) {
    use crate::db::schema::schedules::dsl::*; 
    let result = diesel::delete(schedules.filter(id.eq(task_id)))
        .execute(db_cnxn)
    ;
    match result {
        Ok(num_affected) => {
            if num_affected >= 1 {
                (ClientResponseMessage::Success, 0)
            } else {
                (ClientResponseMessage::Unprocessable(format!("No records found for task_id {task_id}")), 0)
            }
        },
        Err(e) => {
            let msg = format!("Failed to convert data to JSON string. Got error: {}",e.to_string());
            (ClientResponseMessage::ServerError(msg), 0)
        }
    }

}

#[cfg(test)]
mod tests {

    use super::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

    fn setup_db() -> SqliteConnection{
        let mut cnxn = SqliteConnection::establish(":memory:").unwrap();
        cnxn.run_pending_migrations(MIGRATIONS).unwrap();
        cnxn
    }

    #[test]
    fn test_create_task_payload_happy(){
        let args = vec![
            r#"{"task_name": "echo hello","task_type": "PROCESS","command": "echo","args": "hello","cron": "0 3 * * * *","next_run_timestamp": 1720313744}"#.to_string()
        ];
        assert!(create_task_payload(args).is_ok())
    }

    #[test]
    fn test_create_task_payload_invalid_json(){
        let args = vec![
            r#"{"task_name": "#.to_string()
        ];
        assert!(create_task_payload(args).is_err())
    }

    #[test]
    fn test_create_task_payload_valid_json_but_not_task(){
        let args = vec![
            r#"{"task_name": "missing all other props"}"#.to_string()
        ];
        assert!(create_task_payload(args).is_err())
    }

    #[test]
    fn test_delete_task_happy(){
        let args = vec!["1".to_string()];
        assert!(delete_task_payload(args).is_ok())
    }

    #[test]
    fn test_delete_task_invalid(){
        let args = vec!["not_an_int".to_string()];
        assert!(delete_task_payload(args).is_err())
    }

    #[test]
    fn test_handle_create_task_happy(){
        let mut db_cnxn = setup_db();
        let task = NewScheduledTask {
            task_name: "echo hello".to_string(),
            task_type: "PROCESS".to_string(),
            command: "echo".to_string(),
            args: "hello".to_string(),
            cron: "0 3 * * * *".to_string(),
            next_run_timestamp: 1720313744
        };
        assert_eq!(handle_create_task(&mut db_cnxn, task), (ClientResponseMessage::Success, 0))
    }

    #[test]
    fn test_handle_list_tasks_empty_db(){
        let mut db_cnxn = setup_db();
        assert_eq!(handle_list_tasks(&mut db_cnxn), (ClientResponseMessage::SuccessWithPayload("[]".to_string()), 0))
    }

    #[test]
    fn test_handle_list_tasks_1_in_db(){
        let mut db_cnxn = setup_db();
        let task = NewScheduledTask {
            task_name: "echo hello".to_string(),
            task_type: "PROCESS".to_string(),
            command: "echo".to_string(),
            args: "hello".to_string(),
            cron: "0 3 * * * *".to_string(),
            next_run_timestamp: 1720313744
        };
        handle_create_task(&mut db_cnxn, task);
        
        let (resp, exit_code) = handle_list_tasks(&mut db_cnxn);
        assert_eq!(exit_code, 0);
        match resp {
            ClientResponseMessage::SuccessWithPayload(json_str) => {
                let tasks: Vec<ScheduledTask> = serde_json::from_str(&json_str).unwrap();
                assert_eq!(tasks.len(), 1);
            },
            _ => panic!("Expected SuccessWithPayload but got {:?}", resp)
        }
    }

    #[test]
    fn test_handle_delete_task_happy(){
        let mut db_cnxn = setup_db();
        let task = NewScheduledTask {
            task_name: "echo hello".to_string(),
            task_type: "PROCESS".to_string(),
            command: "echo".to_string(),
            args: "hello".to_string(),
            cron: "0 3 * * * *".to_string(),
            next_run_timestamp: 1720313744
        };
        handle_create_task(&mut db_cnxn, task);
        assert_eq!(handle_delete_task(&mut db_cnxn, 1), (ClientResponseMessage::Success, 0))
    }

    #[test]
    fn test_handle_delete_task_not_found(){
        let mut db_cnxn = setup_db();
        let task = NewScheduledTask {
            task_name: "echo hello".to_string(),
            task_type: "PROCESS".to_string(),
            command: "echo".to_string(),
            args: "hello".to_string(),
            cron: "0 3 * * * *".to_string(),
            next_run_timestamp: 1720313744
        };
        handle_create_task(&mut db_cnxn, task);
        assert_eq!(handle_delete_task(&mut db_cnxn, 2), (ClientResponseMessage::Unprocessable("No records found for task_id 2".to_string()), 0))
    }

}