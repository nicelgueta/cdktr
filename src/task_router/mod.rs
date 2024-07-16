use crate::{
    exceptions::GenericError,
    models::{AgentConfig, Task},
    server::agent::AgentRequest,
    utils::AsyncQueue,
    zmq_helpers::{get_agent_tcp_uri, get_zmq_req},
};
use std::collections::HashMap;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio::time::sleep;
use zeromq::SocketSend;

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
    live_agents: Arc<Mutex<HashMap<String, AgentConfig>>>,
}

impl TaskRouter {
    pub fn new(
        task_router_queue: AsyncQueue<Task>,
        live_agents: Arc<Mutex<HashMap<String, AgentConfig>>>,
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
                Ok(agent_id) => {
                    let mut req = get_zmq_req(&get_agent_tcp_uri(&agent_id)).await;
                    req.send(AgentRequest::Run(task).into())
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
    /// asks the server which agent is available
    /// and return the agent_id of an agent that is
    /// suitable to execute the flow.
    /// If no agents are available it fires to the queue of the first agent.
    /// TODO: change to use other logic to determine which agent to queue it up to.
    /// This should be based on executor type but for now since there's only one type
    /// we'll leave this as just taking the first one
    async fn find_agent(&self) -> Result<String, GenericError> {
        let ag_mut = self.live_agents.lock().await;
        if ag_mut.is_empty() {
            return Err(GenericError::MissingAgents);
        };
        for (agent_id, config) in ag_mut.iter() {
            if !config.get_max_threads_reached() {
                return Ok(agent_id.clone());
            }
        }
        Ok(ag_mut.iter().next().unwrap().0.clone())
    }
}

#[cfg(test)]
mod tests {}
