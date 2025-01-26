use crate::db::update_next_timestamp;
use crate::db::{
    get_queueable_schedules,
    models::{ScheduledTask, ToTask},
};
use crate::models::traits::EventListener;
use crate::models::Task;
use crate::utils::data_structures::AsyncQueue;
use async_trait::async_trait;
use chrono::Utc;
use cron::Schedule;
use diesel::SqliteConnection;
use log::{debug, info};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use std::{collections::VecDeque, sync::Arc};
use tokio::sync::Mutex;
use tokio::time::sleep;

/// Main scheduling component. This component has an internal task queue for tasks
/// that are to be scheduled within the next poll interval.
/// There are two main loops that this component runs.
/// The first is to check the time of the first item in the queue and wait.
/// Once time, the scheduler dequeues the task and sends it to the taskmanager.
/// The second loop runs on a separate thread (because currently cannot find a way
/// to read diesel async) to poll the DB for schedules and when it finds flows that
/// are supposed to start within the next poll interval it queues them in order of
/// earliest to latest
pub struct Scheduler {
    db_cnxn: Arc<Mutex<SqliteConnection>>,
    poll_interval_seconds: i32,
    task_queue: Arc<Mutex<VecDeque<ScheduledTask>>>,
}

#[async_trait]
impl EventListener<Task> for Scheduler {
    async fn start_listening(&mut self, out_queue: AsyncQueue<Task>) {
        let task_queue = self.task_queue.clone();
        let poll_interval_seconds = self.poll_interval_seconds.clone();
        let db_cnxn_cl = self.db_cnxn.clone();
        tokio::spawn(async move {
            // None passed to kill_after to ensure the loop never ends
            poll_db_loop(
                task_queue,
                db_cnxn_cl,
                Utc::now().timestamp() as i32,
                poll_interval_seconds,
                None,
            )
            .await
        });
        info!("SCHEDULER: Starting main scheduler loop");
        self.main_loop(out_queue).await;
    }
}

impl Scheduler {
    pub fn _new(db_cnxn: Arc<Mutex<SqliteConnection>>, poll_interval_seconds: i32) -> Self {
        Self {
            db_cnxn,
            poll_interval_seconds,
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    async fn main_loop(&mut self, mut out_queue: AsyncQueue<Task>) {
        loop {
            let first_task_r = {
                let mut internal_task_q = self.task_queue.lock().await;
                internal_task_q.pop_front()
            };
            let sched_task = match first_task_r {
                Some(sched_task) => {
                    if first_task_is_ready(&sched_task) {
                        sched_task
                    } else {
                        sleep(Duration::from_millis(10)).await;
                        continue;
                    }
                }
                None => {
                    sleep(Duration::from_millis(10)).await;
                    continue;
                }
            };
            let task = sched_task.to_task();
            debug!("SCHEDULER: found task - adding to out queue");
            out_queue.put(task).await
        }
    }
}
async fn poll_db_loop(
    task_queue: Arc<Mutex<VecDeque<ScheduledTask>>>,
    db_cnxn: Arc<Mutex<SqliteConnection>>,
    start_timestamp: i32,
    poll_interval_seconds: i32,
    kill_after: Option<i32>,
) {
    while (Utc::now().timestamp() as i32) < start_timestamp {
        thread::sleep(Duration::from_millis(10));
    }
    // use start + poll interval in order not to accidentally overlap schedules with
    // minor microsecond differences
    let mut current_timestamp = start_timestamp;
    loop {
        if let Some(kill_after) = kill_after {
            if current_timestamp - start_timestamp >= kill_after {
                break;
            }
        };
        let secs = Duration::from_secs(poll_interval_seconds as u64);
        thread::sleep(secs);
        {
            let mut cnxn = db_cnxn.lock().await;
            poll_db(
                task_queue.clone(),
                &mut cnxn,
                current_timestamp,
                poll_interval_seconds,
            )
            .await;
        }
        current_timestamp += poll_interval_seconds as i32;
    }
    info!("Polling loop has ended");
}
async fn poll_db(
    task_queue: Arc<Mutex<VecDeque<ScheduledTask>>>,
    cnxn: &mut diesel::SqliteConnection,
    current_timestamp: i32,
    poll_interval_seconds: i32,
) {
    let scheds = get_queueable_schedules(cnxn, current_timestamp, poll_interval_seconds);
    if scheds.len() < 1 {
        return;
    } else {
        let mut task_q_mutex = task_queue.lock().await;
        for task in scheds {
            (*task_q_mutex).push_back(task.clone());
            match &task.cron {
                Some(cron) => {
                    let schedule = Schedule::from_str(cron).unwrap();
                    let next_run_ts = schedule.upcoming(Utc).next().unwrap().timestamp() as i32;
                    update_next_timestamp(cnxn, task.id, next_run_ts).expect(&format!(
                        "SCHEDULER: failed to update next run timestamp for {}",
                        &task.id
                    ));
                }
                None => (),
            }
        }
    }
}

fn first_task_is_ready(first_task: &ScheduledTask) -> bool {
    (Utc::now().timestamp() as i32) >= first_task.next_run_timestamp
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use std::{collections::VecDeque, sync::Arc};
    use tokio::sync::Mutex;

    use super::{first_task_is_ready, poll_db, poll_db_loop};
    use crate::db::{
        get_connection,
        models::{NewScheduledTask, ScheduledTask},
    };

    use diesel::RunQueryDsl;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");
    /// helper macro to provide a nice syntax to generate the database models from a
    /// json token tree
    macro_rules! model_from_json {
        ($model:ty, $($json:tt)+) => {
            serde_json::from_value::<$model>(
                serde_json::json!($($json)+)
            ).expect(&format!("Failed to create {} from json tt", stringify!($model) ))
        };
    }

    #[tokio::test]
    async fn test_first_is_ready() {
        let mut q: VecDeque<ScheduledTask> = VecDeque::new();
        let curr_timestamp = Utc::now().timestamp() as i32;
        let tasks = [
            ScheduledTask {
                id: 1,
                task_name: String::from("Task 1"),
                task_type: String::from("PROCESS"),
                command: String::from("echo"),
                args: Some(String::from("'Hello, World!'")),
                cron: Some(String::from("0 5 * * * *")),
                timestamp_created: curr_timestamp,
                next_run_timestamp: curr_timestamp,
            },
            ScheduledTask {
                id: 2,
                task_name: String::from("Task 2"),
                task_type: String::from("PROCESS"),
                command: String::from("ls"),
                args: Some(String::from("-la")),
                cron: Some(String::from("0 6 * * * *")),
                timestamp_created: curr_timestamp,
                next_run_timestamp: curr_timestamp + 10000, // won't be ready
            },
            ScheduledTask {
                id: 3,
                task_name: String::from("Task 3"),
                task_type: String::from("PROCESS"),
                command: String::from("backup"),
                args: Some(String::from("--all")),
                cron: Some(String::from("0 7 * * * *")),
                timestamp_created: curr_timestamp,
                next_run_timestamp: curr_timestamp + 10000, // won't be ready
            },
        ];
        for task in tasks {
            q.push_back(task);
        }

        assert!(first_task_is_ready(&q.pop_front().unwrap()));
    }

    #[tokio::test]
    async fn test_poll_db() {
        use crate::db::schema::schedules;
        let task_queue: Arc<Mutex<VecDeque<ScheduledTask>>> = Arc::new(Mutex::new(VecDeque::new()));
        let mut cnxn = get_connection(None);
        cnxn.run_pending_migrations(MIGRATIONS).unwrap();

        // load some dummy data
        let curr = Utc::now().timestamp() as i32;
        let poll_interval_seconds = 5;
        let schedule_json = model_from_json!(Vec<NewScheduledTask>, [
            {
                "task_name": "echo hello",
                "task_type": "PROCESS",
                "command": "echo",
                "args": "hello",
                "cron": "0 3 * * * *", // these don't correspond - ignore as not used for this
                "next_run_timestamp": curr + 2 // should be found
            },
            {
                "task_name": "Echo World",
                "task_type": "PROCESS",
                "command": "echo",
                "args": "world",
                "cron": "0 4 * * * *", // these don't correspond - ignore as not used for this
                "next_run_timestamp": curr + 3 // should be queued
            },
            {
                "task_name": "Echo Jelly",
                "task_type": "PROCESS",
                "command": "echo",
                "args": "jelly",
                "cron": "0 5 * * * *", // these don't correspond - ignore as not used for this
                "next_run_timestamp": curr + 10 // should not be queued
            }
        ]);
        diesel::insert_into(schedules::table)
            .values(&schedule_json)
            .execute(&mut cnxn)
            .expect("Failed to execute insert for schedules");

        // runn the test
        poll_db(task_queue.clone(), &mut cnxn, curr, poll_interval_seconds).await;

        let task_queue = task_queue.lock().await;
        assert_eq!(task_queue.len(), 2);
    }

    #[tokio::test]
    async fn test_poll_db_loop() {
        /// Test the loop by running a poll interval of 1 second but running for 5 seconds.
        /// This means that 2 of the 3 scheduled tasks should be picked up since they're all <= 5
        /// seconds of the total duration of the loop
        use crate::db::schema::schedules;
        let task_queue: Arc<Mutex<VecDeque<ScheduledTask>>> = Arc::new(Mutex::new(VecDeque::new()));
        let mut cnxn = get_connection(None);
        cnxn.run_pending_migrations(MIGRATIONS).unwrap();

        // load some dummy data
        let curr = Utc::now().timestamp() as i32;
        let poll_interval_seconds = 1;
        let schedule_json = model_from_json!(Vec<NewScheduledTask>, [
            {
                "task_name": "echo 0",
                "task_type": "PROCESS",
                "command": "echo",
                "args": "0",
                "cron": "0 3 * * * *", // these don't correspond - ignore as not used for this
                "next_run_timestamp": curr // should be found
            },
            {
                "task_name": "Echo 1",
                "task_type": "PROCESS",
                "command": "echo",
                "args": "1",
                "cron": "0 4 * * * *", // these don't correspond - ignore as not used for this
                "next_run_timestamp": curr + 1 // should be queued
            },
            {
                "task_name": "Echo 2",
                "task_type": "PROCESS",
                "command": "echo",
                "args": "2",
                "cron": "0 4 * * * *", // these don't correspond - ignore as not used for this
                "next_run_timestamp": curr + 2 // should be queued
            },

            // these should not be queued as test should end once hit 3 second mark
            {
                "task_name": "Echo 3",
                "task_type": "PROCESS",
                "command": "echo",
                "args": "3",
                "cron": "0 5 * * * *", // these don't correspond - ignore as not used for this
                "next_run_timestamp": curr + 3 // should not be queued
            },
            {
                "task_name": "Echo 4",
                "task_type": "PROCESS",
                "command": "echo",
                "args": "4",
                "cron": "0 5 * * * *", // these don't correspond - ignore as not used for this
                "next_run_timestamp": curr + 4 // should not be queued
            },
            {
                "task_name": "Echo 5",
                "task_type": "PROCESS",
                "command": "echo",
                "args": "5",
                "cron": "0 5 * * * *", // these don't correspond - ignore as not used for this
                "next_run_timestamp": curr + 5 // should not be queued
            }
        ]);
        diesel::insert_into(schedules::table)
            .values(&schedule_json)
            .execute(&mut cnxn)
            .expect("Failed to execute insert for schedules");

        // runn the test
        poll_db_loop(
            task_queue.clone(),
            Arc::new(Mutex::new(cnxn)),
            curr,
            poll_interval_seconds,
            Some(3),
        )
        .await;

        let task_queue = task_queue.lock().await;
        assert_eq!(task_queue.len(), 3);
        for i in 0..3 {
            assert_eq!(task_queue[i].args, Some(i.to_string()));
        }
    }
}
