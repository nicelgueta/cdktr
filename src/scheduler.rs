
/// Main scheduling component. This component has an internal task queue for tasks
/// that are to be scheduled within the next poll interval.
/// There are two main loops that this component runs.
/// The first is to check the time of the first item in the queue and wait.
/// Once time, the scheduler dequeues the task and sends it to the taskmanager.
/// The second loop runs on a separate thread (because currently cannot find a way
/// to read diesel async) to poll the DB for schedules and when it finds flows that
/// are supposed to start within the next poll interval it queues them in order of 
/// earliest to latest
/// 

use std::{collections::VecDeque, sync::Arc};
use std::thread;
use std::time::Duration;
use tokio::sync::{
    Mutex,
    mpsc::Sender
};
use tokio::time::sleep;
use cron::Schedule;
use std::str::FromStr;
use crate::db::update_next_timestamp;
use crate::db::{
    get_connection, get_queueable_schedules,
    models::{ScheduledTask, ToTask}
};
use crate::models::Task;
use chrono::Utc;
pub struct Scheduler {
    database_url: Option<String>,
    poll_interval_seconds: i32,
    pub task_queue: Arc<Mutex<VecDeque<ScheduledTask>>>,
}
impl Scheduler {
    pub fn new(
        database_url: Option<String>, 
        poll_interval_seconds: i32,
    ) -> Self {
        Self {
            database_url, 
            poll_interval_seconds, 
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
    pub async fn start(&mut self, sender: Sender<Task>) {
        let task_queue = self.task_queue.clone();
        let database_url = self.database_url.clone();
        let poll_interval_seconds = self.poll_interval_seconds.clone();
        let mut cnxn = get_connection(database_url.as_deref());
        tokio::spawn(async move {
            // None passed to kill_after to ensure the loop never ends
            poll_db_loop(
                task_queue, 
                &mut cnxn, 
                Utc::now().timestamp() as i32,
                poll_interval_seconds, 
                None
            ).await
        });
        println!("SCHEDULER: Starting main scheduler loop");
        self.main_loop(sender).await;
    }

    async fn main_loop(&mut self, sender: Sender<Task>) {
        loop {
            {
                let internal_task_q = self.task_queue.lock().await;
                while ! first_task_is_ready(&*internal_task_q) {
                    // just hang while we wait for the first task to be ready to send
                    // to TM
                    sleep(Duration::from_millis(10)).await;
                }
            }
            {
                let sched_task = self.task_queue.lock().await.pop_front().expect(
                    "Unable to find a task at the front of the queue"
                );

                let task = sched_task.to_task();
                sender.send(task).await.expect("Failed to send ScheduledTask to TaskRouter");
            }
        }
    }

}
async fn poll_db_loop(
    task_queue: Arc<Mutex<VecDeque<ScheduledTask>>>, 
    cnxn: &mut diesel::SqliteConnection,
    start_timestamp: i32,
    poll_interval_seconds: i32,
    kill_after: Option<i32>
) {
    while (Utc::now().timestamp() as i32) < start_timestamp {
        thread::sleep(Duration::from_millis(10));
    };
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
        poll_db(task_queue.clone(), cnxn, current_timestamp, poll_interval_seconds).await;
        current_timestamp += poll_interval_seconds as i32;
    }
    println!("Polling loop has ended");
}
async fn poll_db(
    task_queue: Arc<Mutex<VecDeque<ScheduledTask>>>,
    cnxn: &mut diesel::SqliteConnection,
    current_timestamp: i32,
    poll_interval_seconds: i32
) {
    let scheds = get_queueable_schedules(
        cnxn, 
        current_timestamp,
        poll_interval_seconds
    );
    if scheds.len() < 1 {
        return ;
    } else {
        let mut task_mutex = task_queue.lock().await;
        for task in scheds {
            (*task_mutex).push_back(task.clone());
            match &task.cron {
                Some(cron) => {
                    let schedule = Schedule::from_str(cron).unwrap();
                    let next_run_ts = schedule.upcoming(Utc).next().unwrap().timestamp() as i32;
                    update_next_timestamp(cnxn, task.id, next_run_ts).expect(
                        &format!("SCHEDULER: failed to update next run timestamp for {}", &task.id)
                    );
                },
                None => ()
            }
            
        }

    }
}


fn first_task_is_ready(task_q: &VecDeque<ScheduledTask>) -> bool {
    if (task_q).len() == 0 {
        false
    } else {
        let first_task = &task_q[0];
        (Utc::now().timestamp() as i32) >= first_task.next_run_timestamp
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, sync::Arc};
    use chrono::Utc;
    use tokio::sync::Mutex;

    use crate::db::{
        get_connection,
        models::{ScheduledTask, NewScheduledTask}
    };
    use super::{first_task_is_ready, poll_db, poll_db_loop};

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
                args: Some(String::from("Hello, World!")),
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
        };

        assert!(first_task_is_ready(&q));
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
            &mut cnxn, 
            curr,
            poll_interval_seconds, 
            Some(3)
        ).await;

        let task_queue = task_queue.lock().await;
        assert_eq!(task_queue.len(), 3);
        for i in 0..3 {
            assert_eq!(task_queue[i].args, Some(i.to_string()));
        }

    }
}