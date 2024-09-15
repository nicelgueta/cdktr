use crate::{
    exceptions::GenericError,
    models::{AgentMeta, AgentPriorityQueue, Task},
    server::agent::AgentAPI,
    utils::AsyncQueue,
    zmq_helpers::{get_agent_tcp_uri, get_zmq_req},
};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use zeromq::SocketSend;

/// The Task Router is responsible for distributing tasks to agent workers.
/// It is implemented using a max-heap (priority queue) of AgentMeta, 
/// prioritised by the capacity of tasks that the agent can handle. Tasks
/// are stored on an AsyncQueue
pub struct TaskRouter {
    queue: AsyncQueue<Task>,
    live_agents: AgentPriorityQueue,
}

impl TaskRouter {
    pub fn new(
        task_router_queue: AsyncQueue<Task>,
        live_agents: AgentPriorityQueue,
    ) -> Self {
        Self {
            queue: task_router_queue,
            live_agents,
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
            let agent_res = self.find_agent().await;
            match agent_res {
                Ok(agent_meta) => {
                    let mut req = get_zmq_req(&get_agent_tcp_uri(&agent_meta.agent_id)).await;
                    req.send(AgentAPI::Run(task).into())
                        .await
                        .expect("ZMQ error whle sending msg")
                }
                Err(e) => match e {
                    GenericError::MissingAgents => {
                        println!("Failed to execute task as no agents are running")
                    }
                    _ => panic!("Critical error - expected MissingAgents error"),
                },
            }
        }
    }
    /// returns the next agent from the priority queue
    async fn find_agent(&mut self) -> Result<AgentMeta, GenericError> {
        if self.live_agents.is_empty().await {
            return Err(GenericError::MissingAgents)
        };
        self.live_agents.pop().await
    }
}

#[cfg(test)]
mod tests {}
