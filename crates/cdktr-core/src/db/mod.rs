use diesel::{sqlite::SqliteConnection, Connection, ExpressionMethods, SelectableHelper};
use models::ScheduledTask;

use diesel::prelude::*;
pub mod models;
pub mod schema;

pub fn get_connection(database_url: Option<&str>) -> SqliteConnection {
    let db_url = database_url.unwrap_or(":memory:");
    let mut cnxn = SqliteConnection::establish(db_url)
        .expect(&format!("Failed to establish connection to {}", db_url));
    if db_url == ":memory:" {
        use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
        pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/db/migrations");
        cnxn.run_pending_migrations(MIGRATIONS).unwrap();
    };
    cnxn
}

pub fn get_queueable_schedules(
    cnxn: &mut SqliteConnection,
    current_timestamp: i64,
    poll_interval: u64,
) -> Vec<ScheduledTask> {
    // used in order to used the field names directly as variables
    use self::schema::tasks::dsl::*;

    let nxt_ts = current_timestamp + poll_interval as i64;
    let results = tasks
        .filter(next_run_timestamp.lt(nxt_ts))
        .filter(next_run_timestamp.ge(current_timestamp))
        .select(ScheduledTask::as_select())
        .load(cnxn)
        .expect("Failed to query schedules");
    results
}

pub fn update_next_timestamp(
    cnxn: &mut SqliteConnection,
    task_id: i32,
    timestamp: i64,
) -> Result<usize, diesel::result::Error> {
    use self::schema::tasks::dsl::*;
    diesel::update(tasks.filter(id.eq(task_id)))
        .set(next_run_timestamp.eq(timestamp))
        .execute(cnxn)
}

#[cfg(test)]
mod tests {

    use super::models::NewScheduledTask;
    use super::*;
    use chrono::Utc;
    use diesel::RunQueryDsl;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/db/migrations");

    /// helper macro to provide a nice syntax to generate the database models from a
    /// json token tree
    macro_rules! model_from_json {
        ($model:ty, $($json:tt)+) => {
            serde_json::from_value::<$model>(
                serde_json::json!($($json)+)
            ).expect(&format!("Failed to create {} from json tt", stringify!($model) ))
        };
    }

    #[test]
    fn test_insert_query_schedules() {
        use super::schema::tasks;
        let mut cnxn = get_connection(None);
        cnxn.run_pending_migrations(MIGRATIONS).unwrap();
        let current_datetime = Utc::now();
        let curr = current_datetime.timestamp();
        let nxt = curr + 86_400;
        let schedule_json = model_from_json!(Vec<NewScheduledTask>, [
                {
                    "task_name": "echo start",
                    "task_type": "PROCESS",
                    "command": "echo",
                    "args": "start",
                    "cron": "0 3 * * * *",
                    "next_run_timestamp": curr // should be found as is start
                },
                {
                    "task_name": "echo hello",
                    "task_type": "PROCESS",
                    "command": "echo",
                    "args": "hello",
                    "cron": "0 3 * * * *",
                    "next_run_timestamp": curr + 100 // should be found
                },
                {
                    "task_name": "echo nope",
                    "task_type": "PROCESS",
                    "command": "echo",
                    "args": "nope",
                    "cron": "0 3 * * * *",
                    "next_run_timestamp": nxt // should not found as is the exact start of next timestamp window
                },
                {
                    "task_name": "Echo World",
                    "task_type": "PROCESS",
                    "command": "echo",
                    "args": "world",
                    "cron": "0 4 * * 0",
                    "next_run_timestamp": nxt + 10 // should not be found as beyond interval window
                },
                {
                    "task_name": "Echo Jelly",
                    "task_type": "PROCESS",
                    "command": "echo",
                    "args": "jelly",
                    "cron": "0 5 * * 0",
                    "next_run_timestamp": nxt - 10 // should be found
                }
        ]);
        diesel::insert_into(tasks::table)
            .values(&schedule_json)
            .execute(&mut cnxn)
            .expect("Failed to execute insert for schedules");

        // query part
        let results = get_queueable_schedules(&mut cnxn, curr, 86_400);
        assert!(results.len() == 3);
    }

    #[test]
    fn test_update_next_timestamp() {
        use super::schema::tasks;
        let mut cnxn = get_connection(None);
        cnxn.run_pending_migrations(MIGRATIONS).unwrap();

        let curr = Utc::now().timestamp();

        let schedule_json = model_from_json!(Vec<NewScheduledTask>, [
                {
                    "task_name": "echo start",
                    "task_type": "PROCESS",
                    "command": "echo",
                    "args": "start",
                    "cron": "0 3 * * * *",
                    "next_run_timestamp": curr // should be found as is start
                },
        ]);
        diesel::insert_into(tasks::table)
            .values(&schedule_json)
            .execute(&mut cnxn)
            .expect("Failed to execute insert for schedules");
        let new_ts = curr + 32;
        update_next_timestamp(&mut cnxn, 1, new_ts).expect("Failed to update timestamp");

        let mut results: Vec<ScheduledTask> = tasks::dsl::tasks
            .filter(tasks::id.eq(1))
            .select(ScheduledTask::as_select())
            .load(&mut cnxn)
            .expect("Failed to query schedules");

        assert!(results.len() == 1);
        assert!(results.pop().unwrap().next_run_timestamp == new_ts);
    }
}
