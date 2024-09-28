use crate::{
    exceptions::GenericError,
    models::{AgentMeta, AgentPriorityQueue, Task},
    server::{agent::AgentAPI, models::ClientResponseMessage, traits::API},
    utils::AsyncQueue,
    zmq_helpers::{get_server_tcp_uri, DEFAULT_TIMEOUT},
};
use log::{debug, error};

/// The Task Router is responsible for distributing tasks to agent workers.
/// It is implemented using a max-heap (priority queue) of AgentMeta,
/// prioritised by the capacity of tasks that the agent can handle. Tasks
/// are stored on an AsyncQueue
pub struct TaskRouter {
    queue: AsyncQueue<Task>,
    live_agents: AgentPriorityQueue,
}

impl TaskRouter {
    pub fn new(task_router_queue: AsyncQueue<Task>, live_agents: AgentPriorityQueue) -> Self {
        Self {
            queue: task_router_queue,
            live_agents,
        }
    }
    /// Main loop listening on messages received from the scheduler and external
    /// event listeners that send Tasks for execution
    pub async fn start(&mut self) {
        loop {
            let task = self.queue.get_wait().await;
            let agent_res = self.find_agent().await;
            match agent_res {
                Ok(agent_meta) => {
                    match AgentAPI::Run(task)
                        .send(
                            &get_server_tcp_uri(&agent_meta.host, agent_meta.port),
                            DEFAULT_TIMEOUT,
                        )
                        .await
                    {
                        Ok(msg) => {
                            match msg {
                                ClientResponseMessage::Success => {
                                    debug!("Successfully submitted task to agent")
                                }
                                cli_msg => {
                                    let str_msg: String = cli_msg.into();
                                    error!(
                                        "Failed to execute task on agent {}. Got message {}",
                                        agent_meta.agent_id(),
                                        str_msg
                                    )
                                }
                            }
                        }
                        Err(e) => match e {
                            GenericError::TimeoutError => {
                                error!("Timed out wating on response from agent for task execution")
                            }
                            e => error!("Failed to execute task. Error: {}", e.to_string()),
                        },
                    };
                    // return the agent meta back to queue
                    self.return_agent_meta(agent_meta).await
                }
                Err(e) => match e {
                    GenericError::MissingAgents => {
                        error!("Failed to execute task as no agents are running")
                    }
                    _ => panic!("Critical error - expected MissingAgents error"),
                },
            }
        }
    }
    /// returns metadata object for the agent with the highest capacity
    /// from the priority queue
    async fn find_agent(&mut self) -> Result<AgentMeta, GenericError> {
        if self.live_agents.is_empty().await {
            return Err(GenericError::MissingAgents);
        };
        self.live_agents.pop().await
    }

    /// retuns the agent meta back to the queue
    async fn return_agent_meta(&mut self, agent_meta: AgentMeta) {
        self.live_agents.push(agent_meta).await
    }
}

#[cfg(test)]
mod tests {}
