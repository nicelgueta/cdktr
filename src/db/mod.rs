use diesel::{sqlite::SqliteConnection, Connection, ExpressionMethods, SelectableHelper};
use models::ScheduledTask;
use std::env;

use diesel::prelude::*;
pub mod models;
pub mod schema;

macro_rules! model_from_json {
    ($model:ty, $($json:tt)+) => {
        serde_json::from_value::<$model>(
            serde_json::json!($($json)+)
        ).expect(&format!("Failed to create {} from json", stringify!($model) ))
    };
}


pub fn get_connection(database_url: Option<&str>) -> SqliteConnection {
    let db_url = database_url.unwrap_or(":memory:");
    SqliteConnection::establish(db_url).expect(
        &format!("Failed to establish connection to {}", db_url)
    )

}

pub fn get_schedules(
    cnxn: &mut SqliteConnection, 
    current_timestamp: i32, 
    poll_interval: i32
) -> Vec<ScheduledTask> {
    
    // used in order to used the field names directly as variables
    use self::schema::schedules::dsl::*; 

    let nxt_ts = current_timestamp + poll_interval;
    let results = schedules
        .filter(next_run_timestamp.le(nxt_ts))
        .filter(next_run_timestamp.ge(current_timestamp))
        .select(ScheduledTask::as_select())
        .load(cnxn)
        .expect("Failed to query schedules")
    ;
    results
}

#[cfg(test)]
mod tests {

    use super::{get_connection, get_schedules};
    use super::models::ScheduledTask;
    use diesel::RunQueryDsl;
    use chrono::Utc;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

    #[test]
    fn test_insert_query_schedules() {
        use super::schema::schedules;

        let mut cnxn = get_connection(None);
        cnxn.run_pending_migrations(MIGRATIONS).unwrap();
        let current_datetime = Utc::now();
        let curr = current_datetime.timestamp() as i32;
        let nxt = curr + 86_400;
        let schedule_json = model_from_json!(Vec<ScheduledTask>, [
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
        ]);
        diesel::insert_into(schedules::table)
            .values(&schedule_json)
            .execute(&mut cnxn)
            .expect("Failed to execute insert for schedules");

        // query part
        let results = get_schedules(&mut cnxn, curr, 86_400);
        assert!(results.len() == 2);
        
    }

}