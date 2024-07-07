use diesel::prelude::*;
/// API module to provide all of the principal message handling
/// utilities
/// 
use crate::db::models::{NewScheduledTask, ScheduledTask};
use super::models::{ClientConversionError, ClientResponseMessage};
use diesel::RunQueryDsl;

pub fn create_task_from_zmq_args(args: Vec<String>) -> Result<NewScheduledTask, ClientConversionError> {
    if args.len() == 0 {
        Err(ClientConversionError::new(
            "No payload found for CREATETASK command".to_string()
        ))
    } else {
        let parse_res: Result<NewScheduledTask, serde_json::Error> = serde_json::from_str(&args[0]);
        match parse_res{
            Ok(task) => Ok(task),
            Err(e) => Err(ClientConversionError::new(
                format!("Invalid JSON for ScheduledTask. Error: {}", e.to_string())
            ))
        }
    }
}

pub fn handle_create_task(db_cnxn: &mut SqliteConnection, scheduled_task: NewScheduledTask) -> (ClientResponseMessage, bool) {
    use crate::db::schema::schedules;
    let result = diesel::insert_into(schedules::table)
        .values(&scheduled_task)
        .execute(db_cnxn)
    ;
    match result {
        Ok(_v) => (ClientResponseMessage::Success, false),
        Err(e) => (ClientResponseMessage::ServerError(e.to_string()), false)
    }

}

pub fn handle_list_tasks(db_cnxn: &mut SqliteConnection) -> (ClientResponseMessage, bool) {
    use crate::db::schema::schedules::dsl::*; 
    let results: Result<Vec<ScheduledTask>, diesel::result::Error> = schedules
        .select(ScheduledTask::as_select())
        .load(db_cnxn)
    ;
    match results {
        Ok(res) => {
            match serde_json::to_string(&res) {
                Ok(json_str) => (ClientResponseMessage::SuccessWithPayload(json_str), false),
                Err(e) => {
                    (
                        ClientResponseMessage::ServerError(
                            format!("Failed to convert data to JSON string. Got error: {}",e.to_string())
                        ), 
                        false
                    )
                }
            }
        },
        Err(e) => (ClientResponseMessage::ServerError(
            format!("Failed to query data from database. Got error: {}",e.to_string())
        ), false)
    }
}


pub fn handle_delete_task(db_cnxn: &mut SqliteConnection, task_id: i32) -> (ClientResponseMessage, bool) {
    use crate::db::schema::schedules::dsl::*; 
    let result = diesel::delete(schedules.filter(id.eq(task_id)))
        .execute(db_cnxn)
    ;
    match result {
        Ok(_v) => (ClientResponseMessage::Success, false),
        Err(e) => {
            let msg = format!("Failed to convert data to JSON string. Got error: {}",e.to_string());
            (ClientResponseMessage::ServerError(msg), false)
        }
    }

}