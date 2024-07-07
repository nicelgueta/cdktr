use std::collections::VecDeque;

use crate::models::Task;

use super::models::RepReqError;

pub fn create_task_run_payload(args:Vec<String>) -> Result<Task, RepReqError> {
    let task_res = Task::try_from(VecDeque::from(args));
    match task_res {
        Ok(task) => Ok(task),
        Err(e) => Err(RepReqError::ParseError(
            format!("Invalid TASKDEF: {}", e.to_string())
        ))
    }
}