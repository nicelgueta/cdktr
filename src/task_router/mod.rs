use std::{sync::Arc, time::Duration};
use tokio::sync::{mpsc::Receiver, Mutex};
use tokio::time::sleep;
use zeromq::{ReqSocket, Socket};

use crate::{models::Task, utils::AsyncQueue};

/// The Task Router is responsible for distributing tasks to agent workers.
/// It publishes a message on the PUB socket that all workers are listening to
/// and distributes a task to the first agent that responds with available
/// resoruces for execution.
///
/// It is constructed with a shared pointer to the publisher and the receive end
/// of the communication channel that will be used by the scheudler and event listeners
///
pub struct TaskRouter {
    queue: AsyncQueue<Task>,
}

impl TaskRouter {
    pub fn new(task_router_queue: AsyncQueue<Task>) -> Self {
        Self {
            queue: task_router_queue,
        }
    }
    /// Main loop listening on messages received from the scheduler and external
    /// event listeners that send Tasks for execution
    pub async fn start(&mut self) {
        loop {
            let item_r = self.queue.get().await;
            let task = if let Some(t) = item_r {
                t
            } else {
                sleep(Duration::from_micros(10)).await;
                continue;
            };
            // got task - find agents
        }
    }
    /// asks the server which agent is available
    fn find_agents() {}
}
