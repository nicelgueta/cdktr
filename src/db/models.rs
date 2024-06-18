use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Deserialize, Serialize, Debug, Clone, Insertable)]
#[diesel(table_name = crate::db::schema::schedules)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ScheduledTask {
    pub id: i32,
    pub task_name: String,
    pub command: String,
    pub args: String,
    pub cron: String,
    pub timestamp_created: i32,
    pub next_run_timestamp: i32
}

#[derive(Insertable, Deserialize, Serialize)]
#[diesel(table_name = crate::db::schema::schedules)]
pub struct NewScheduledTask {
    pub task_name: String,
    pub command: String,
    pub args: String,
    pub cron: String,
    pub next_run_timestamp: i32
}