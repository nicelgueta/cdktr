use duckdb::Connection;
use chrono::{DateTime, Utc};

#[derive(Debug)]
struct ScheduledTask {
    id: i64,
    task_name: String,
    command: String,
    args: String,
    cron: String,
    date_created: DateTime<Utc>
}


pub fn get_connection(db_file: &str) -> Connection {
    Connection::open(db_file).expect("Unable to open DB file")
}