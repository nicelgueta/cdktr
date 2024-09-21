use crate::{
    exceptions::GenericError,
    models::{AgentMeta, AgentPriorityQueue, Task},
    server::{agent::AgentAPI, models::ClientResponseMessage},
    utils::AsyncQueue,
    zmq_helpers::{get_server_tcp_uri, get_zmq_req, send_recv_with_timeout, DEFAULT_TIMEOUT},
};
use std::time::Duration;
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
                Ok(mut agent_meta) => {
                    let resp = send_recv_with_timeout(
                        get_server_tcp_uri(&agent_meta.host, agent_meta.port),
                        AgentAPI::Run(task).into(),
                        DEFAULT_TIMEOUT,
                    )
                    .await;
                    match resp {
                        Ok(msg) => {
                            let parsed_resp = ClientResponseMessage::from(msg);
                            match parsed_resp {
                                ClientResponseMessage::Success => {
                                    // record the task as running in the agent meta
                                    agent_meta.inc_running_task();
                                }
                                cli_msg => {
                                    let str_msg: String = cli_msg.into();
                                    println!(
                                        "Failed to execute task on agent {}. Got message {}",
                                        agent_meta.agent_id(),
                                        str_msg
                                    )
                                }
                            }
                        }
                        Err(e) => match e {
                            GenericError::TimeoutError => {
                                println!(
                                    "Timed out wating on response from agent for task execution"
                                )
                            }
                            e => println!("Failed to execute task. Error: {}", e.to_string()),
                        },
                    };
                    // return the agent meta back to queue
                    self.return_agent_meta(agent_meta).await
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
