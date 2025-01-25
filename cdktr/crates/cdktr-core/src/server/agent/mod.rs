use async_trait::async_trait;
use log::{debug, error, info, warn};
use std::{env, time::Duration};
use tokio::time::sleep;

use crate::{
    api::{AgentAPI, PrincipalAPI, API},
    exceptions::GenericError,
    models::Task,
    prelude::ClientResponseMessage,
    prelude::CDKTR_DEFAULT_TIMEOUT,
    utils::data_structures::AsyncQueue,
};

use super::traits::Server;
mod helpers;

pub struct AgentServer {
    /// ID of the publisher currently subscribed to
    instance_id: String,
    task_queue: AsyncQueue<Task>,
}

impl AgentServer {
    pub fn new(instance_id: String, task_queue: AsyncQueue<Task>) -> Self {
        // start with an empty string - the first heartbeat from the principal
        //will correct this to the new value
        Self {
            instance_id,
            task_queue,
        }
    }
    pub async fn register_with_principal(
        &self,
        principal_uri: &str,
        max_tasks: usize,
    ) -> Result<(), GenericError> {
        debug!("Registering agent with principal @ {}", &principal_uri);
        let max_reconnection_attempts = env::var("AGENT_RECONNNECT_ATTEMPTS")
            .unwrap_or("5".to_string())
            .parse::<usize>()
            .unwrap_or({
                warn!("Env var AGENT_RECONNNECT_ATTEMPTS specified but is not a valid number - using default 5");
                5
            });
        let mut actual_attempts: usize = 0;
        loop {
            let request = PrincipalAPI::RegisterAgent(self.instance_id.clone(), max_tasks);
            let reconn_result = request.send(principal_uri, CDKTR_DEFAULT_TIMEOUT).await;
            if let Ok(cli_msg) = reconn_result {
                match cli_msg {
                    ClientResponseMessage::Success => {
                        info!("Successfully registered agent with principal");
                        break;
                    }
                    other => {
                        warn!("Non-success message -> {}", {
                            let m: String = other.into();
                            m
                        });
                    }
                }
            } else {
                warn!(
                    "Failed to communicate to principal: {}",
                    reconn_result.unwrap_err().to_string()
                )
            };
            actual_attempts += 1;
            if actual_attempts == max_reconnection_attempts {
                error!("Max reconnect attempts reached - exiting");
                return Err(GenericError::TimeoutError);
            }
            warn!("Unable to reconnect to principal - trying again 5 seconds");
            sleep(Duration::from_secs(5)).await;
        }

        Ok(())
    }
}

#[async_trait]
impl Server<AgentAPI> for AgentServer {
    async fn handle_client_message(&mut self, cli_msg: AgentAPI) -> (ClientResponseMessage, usize) {
        match cli_msg {
            AgentAPI::Ping => (ClientResponseMessage::Pong, 0),
            AgentAPI::Run(task) => {
                self.task_queue.put(task).await;
                (ClientResponseMessage::Success, 0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeromq::ZmqMessage;

    #[tokio::test]
    async fn test_handle_cli_message_all_happy() {
        let test_params = [
            ("PING", ClientResponseMessage::Pong, 0),
            ("RUN|PROCESS|echo|hello", ClientResponseMessage::Success, 0),
        ];
        let mut server = AgentServer::new("newid".to_string(), AsyncQueue::new());
        for (zmq_s, response, exp_exit_code) in test_params {
            let ar = AgentAPI::try_from(ZmqMessage::from(zmq_s))
                .expect("Should be able to unwrap the agent from ZMQ command");
            let (resp, exit_code) = server.handle_client_message(ar).await;
            assert_eq!(response, resp);
            assert_eq!(exit_code, exp_exit_code);
        }
    }
}
