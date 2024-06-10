use rusqlite::{Connection, Result};

use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct ScheduledTask {
    pub id: i64,
    pub task_name: String,
    pub command: String,
    pub args: String,
    pub cron: String,
    pub date_created: DateTime<Utc>
}


pub fn get_connection(db_file: &str) -> Connection {
    Connection::open(db_file).expect("Unable to open DB file")
}