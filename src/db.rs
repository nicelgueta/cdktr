use rusqlite::{Connection};
use serde::{Deserialize, Serialize};

use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: i64,
    pub task_name: String,
    pub command: String,
    pub args: String,
    pub cron: String,
    pub timestamp_created: i64,
    pub next_run_timestamp: i64
}

fn setup(cnxn: &Connection) {
    // create schedules
    cnxn.execute(
        "create table if not exists schedules (
             id integer primary key,
             task_name text not null unique,
             command text not null,
             args text not null,
             cron text not null,
             timestamp_created bigint not null,
             next_run_timestamp bigint not null
         )",
        (),
    ).unwrap();
}

pub struct DbConnection {
    cnxn: Connection
}
impl DbConnection {
    pub fn new(db_filepath: String) -> Self {
        let cnxn = if db_filepath == "in_memory".to_string() {
            Connection::open_in_memory().expect("Unable to open DB connection")
        } else {
            Connection::open(db_filepath).expect("Unable to open DB connection")
        };
        setup(&cnxn);
        Self {cnxn}
    }
    
    /// Queries schedules from the database and optionally returns a vector
    /// of ScheduledTasks that are due for execution between the current datetime
    /// and the next poll time
    pub fn query_schedules(
        &self, 
        current_datetime: DateTime<Utc>, 
        next_datetime: DateTime<Utc>
    ) -> Vec<ScheduledTask> {
        let current_timestamp = current_datetime.timestamp();
        let next_timestamp = next_datetime.timestamp();
        let mut stmt = self.cnxn.prepare(
            "SELECT * FROM schedules 
            where next_run_datetime between ?1 and ?2
        ").unwrap();
        let result = stmt.query_map(
            (current_timestamp, next_timestamp),
            |r| {
                Ok(ScheduledTask {
                    id: r.get(0).unwrap(),
                    task_name: r.get(1).unwrap(),
                    command: r.get(2).unwrap(),
                    args: r.get(3).unwrap(),
                    cron: r.get(4).unwrap(),
                    timestamp_created: r.get(5).unwrap(),
                    next_run_timestamp: r.get(6).unwrap(),
    
                })
            }
        )
            .unwrap()
            .map(|x| x.unwrap())
            .collect::<Vec<ScheduledTask>>();
        result

    }
}

#[cfg(test)]
mod tests {

    use super::{DbConnection, ScheduledTask};
    use chrono::{DateTime, Utc};
    use serde_json;

    #[test]
    fn test_query_schedules() {
        let db_conn = DbConnection::new("in_memory".to_string());
        let current_datetime = Utc::now();
        let curr = current_datetime.timestamp();
        let nxt = curr + 86_400;
        let next_datetime = DateTime::from_timestamp(nxt, 0).unwrap();

        let schedules: Vec<ScheduledTask> = serde_json::from_value(serde_json::json!([
            {
                "id": 1,
                "task_name": "echo hello",
                "command": "echo",
                "args": "hello",
                "cron": "0 3 * * *", // these don't correspond - ignore as not used for this
                "timestamp_created": curr,
                "next_run_timestamp": curr + 100 // should be found
            },
            {
                "id": 2,
                "task_name": "Echo World",
                "command": "echo",
                "args": "world",
                "cron": "0 4 * * 0", // these don't correspond - ignore as not used for this
                "timestamp_created": curr,
                "next_run_timestamp": nxt + 10 // should not be found
            },
            {
                "id": 3,
                "task_name": "Echo Jelly",
                "command": "echo",
                "args": "jelly",
                "cron": "0 5 * * 0", // these don't correspond - ignore as not used for this
                "timestamp_created": curr,
                "next_run_timestamp": nxt - 10 // should be found
            }
        ])).expect("Failed to deserialize");

    }
}