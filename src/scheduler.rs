
use std::thread;
/// Main scheduling component. This component has an internal task queue for tasks
/// that are to be scheduled within the next poll interval.
/// There are two main loops that this component runs.
/// The first is to check the time of the first item in the queue and wait.
/// Once time, the scheduler dequeues the task and sends it to the taskmanager.
/// The second loop runs on a separate thread (because currently cannot find a way
/// to read duckDB async) to poll the DB for schedules and when it finds flows that
/// are supposed to start within the next poll interval it queues them in order of 
/// earliest to latest
/// 

use std::{collections::VecDeque, sync::Arc};
use std::sync::{Mutex as StdMutex, MutexGuard as StdMutexGuard};
use tokio::sync::Mutex as TokioMutex;
use crate::{
    interfaces::Task,
    db::ScheduledTask
};
use chrono::{DateTime, Utc};

struct QueuedScheduledTask {
    scheduled_task: ScheduledTask,
    start_time: DateTime<Utc>
}

struct Scheduler {
    db_path: String,
    poll_interval_seconds: usize,
    task_queue: Arc<StdMutex<VecDeque<QueuedScheduledTask>>>
}
impl Scheduler {
    fn new(db_path: String, poll_interval_seconds: usize) -> Self {
        Self {
            db_path, 
            poll_interval_seconds, 
            task_queue: Arc::new(StdMutex::new(VecDeque::new()))
        }
    }
    pub async fn start(&self, task_manager_queue: Arc<TokioMutex<VecDeque<Task>>>) {
        let task_queue = self.task_queue.clone();
        thread::spawn(|| {
            poll_db(task_queue)
        });
        self.main_loop(task_manager_queue).await;
    }

    async fn main_loop(&self, task_manager_queue: Arc<TokioMutex<VecDeque<Task>>>) {
        // main loop
        loop {
            {
                let internal_task_q = self.task_queue.lock().unwrap();
                while ! first_task_is_ready(&internal_task_q) {
                    // just hang while we wait for the first task to be ready to send
                    // to TM
                }
            }
            {
                let task = self.task_queue.lock().unwrap().pop_front().expect(
                    "Unable to find a task at the front of the queue"
                );
                let mut tm_q_mut = task_manager_queue.lock().await;
                tm_q_mut.push_back(Task{
                    command: task.scheduled_task.command, 
                    args: Some(task.scheduled_task.args.split("|").map(|x|x.to_string()).collect())
                })
            }
        }
    }

}
fn poll_db(task_queue: Arc<StdMutex<VecDeque<QueuedScheduledTask>>>) {;
    // every x seconds
    // query db schedules
    // if time then queue
}

fn first_task_is_ready(task_q: &StdMutexGuard<VecDeque<QueuedScheduledTask>>) -> bool {
    if (*task_q).len() == 0 {
        false
    } else {
        let first_task = &task_q[0];
        Utc::now() >= first_task.start_time
    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_first_is_ready() {

    }
}