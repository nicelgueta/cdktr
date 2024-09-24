use crate::{
    models::{traits::EventListener, Task},
    utils::AsyncQueue,
};
use log::debug;
use std::sync::Arc;
use tokio::sync::Mutex;

mod scheduler;

/// Spawns the Scheduler in a separate coroutine
async fn spawn_scheduler(
    db_cnxn: Arc<Mutex<diesel::SqliteConnection>>,
    poll_interval_seconds: i32,
    task_router_queue: AsyncQueue<Task>,
) -> tokio::task::JoinHandle<()> {
    debug!("Spawning scheduler");
    let handle = tokio::spawn(async move {
        let mut sched = scheduler::Scheduler::new(db_cnxn, poll_interval_seconds);
        sched.start_listening(task_router_queue).await
    });
    debug!("Scheduler spawned");
    handle
}
