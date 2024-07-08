use crate::{
    models::{Task, ZMQArgs},
    server::models::RepReqError,
};

pub fn create_task_run_payload(args: Vec<String>) -> Result<Task, RepReqError> {
    let task_res = Task::try_from(ZMQArgs::from(args));
    match task_res {
        Ok(task) => Ok(task),
        Err(e) => Err(RepReqError::ParseError(format!(
            "Invalid TASKDEF: {}",
            e.to_string()
        ))),
    }
}
