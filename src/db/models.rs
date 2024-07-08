use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    models::Task,
    utils::arg_str_to_vec
};
use crate::executors::ProcessTask;


pub trait ToTask {
    fn to_task(&self) -> Task ;
}

#[derive(Queryable, Selectable, Deserialize, Serialize, Debug, Clone, Insertable)]
#[diesel(table_name = crate::db::schema::schedules)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ScheduledTask {
    pub id: i32,
    pub task_name: String,
    pub task_type: String,
    pub command: String,
    pub args: Option<String>,
    pub cron: Option<String>,
    pub timestamp_created: i32,
    pub next_run_timestamp: i32
}
impl ToTask for ScheduledTask {
    fn to_task(&self) -> Task {
        match &self.task_type[..] {
            "PROCESS" => {
                let args = if let Some(astr) = &self.args {
                    Some(arg_str_to_vec(astr.to_string()).into())
                } else {
                    None
                };
                let p_task = ProcessTask {
                    command: self.command.clone(),
                    args: args
                };
                Task::Process(p_task)
            },
            other => panic!("Got unsupported scheduled task: {other}")
        }
    }
}

#[derive(Insertable, Deserialize, Serialize)]
#[diesel(table_name = crate::db::schema::schedules)]
pub struct NewScheduledTask {
    pub task_name: String,
    pub task_type: String,
    pub command: String,
    pub args: String,
    pub cron: String,
    pub next_run_timestamp: i32
}