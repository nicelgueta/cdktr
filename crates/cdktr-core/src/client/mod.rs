use log::{debug, error, info, trace, warn};
use std::{env, time::Duration};
use tokio::time::sleep;

use crate::{
    api::{PrincipalAPI, API},
    exceptions::GenericError,
    models::Task,
    prelude::{ClientResponseMessage, CDKTR_DEFAULT_TIMEOUT},
    utils::data_structures::AsyncQueue,
};

mod helpers;

const RETRY_ATTEMPTS: usize = 10;
const RETRY_INTERVAL_MS: u64 = 3000;

/// This client is used to house utility functions at a slightly higher level than the raw API
/// implemented by the PrincipalAPI.
pub struct PrincipalClient {
    /// ID of the principal currently subscribed to
    instance_id: String,
}

impl PrincipalClient {
    pub fn new(instance_id: String) -> Self {
        Self { instance_id }
    }
    pub async fn register_with_principal(&self, principal_uri: &str) -> Result<(), GenericError> {
        debug!("Registering agent with principal @ {}", &principal_uri);
        let max_reconnection_attempts: usize = env::var("AGENT_RECONNNECT_ATTEMPTS")
            .unwrap_or(RETRY_ATTEMPTS.to_string())
            .parse()
            .unwrap();
        let mut actual_attempts: usize = 0;
        loop {
            let request = PrincipalAPI::RegisterAgent(self.instance_id.clone());
            let reconn_result = request.send(principal_uri, CDKTR_DEFAULT_TIMEOUT).await;
            if let Ok(cli_msg) = reconn_result {
                match cli_msg {
                    ClientResponseMessage::Success => {
                        info!("Successfully registered agent with principal");
                        break;
                    }
                    other => {
                        warn!("Non-success message -> {}", other.to_string());
                    }
                }
            } else {
                actual_attempts += 1;
                if actual_attempts == max_reconnection_attempts {
                    error!(
                        "Max reconnect attempts reached - connection with principal has been lost"
                    );
                    return Err(GenericError::TimeoutError);
                }
                warn!(
                    " Failed to communicate to principal: {} - trying again in {} ms (attempt {} of {})",
                    reconn_result.unwrap_err().to_string(),
                    RETRY_INTERVAL_MS.to_string(),
                    actual_attempts,
                    max_reconnection_attempts
                );
                sleep(Duration::from_millis(RETRY_INTERVAL_MS)).await;
            }
        }

        Ok(())
    }

    pub async fn process_fetch_task(
        &self,
        task_queue: &mut AsyncQueue<Task>,
        principal_uri: &str,
        timeout: Duration,
    ) -> Result<(), GenericError> {
        let request = PrincipalAPI::FetchTask(self.instance_id.clone());
        match request.send(&principal_uri, timeout).await {
            Ok(cli_resp) => match cli_resp {
                ClientResponseMessage::Success => {
                    trace!("No work on global task queue - waiting");
                    Ok(())
                }
                ClientResponseMessage::SuccessWithPayload(task_str) => {
                    info!("Task delivered from Principal -> {}", &task_str);
                    let task_res = Task::try_from(task_str);
                    match task_res {
                        Ok(task) => {
                            task_queue.put(task).await;
                            Ok(())
                        }
                        Err(e) => Err(GenericError::ZMQParseError(e)),
                    }
                }
                other => Err(GenericError::RuntimeError(format!(
                    "Unexpected client response message received from principal: {}",
                    other.to_string()
                ))),
            },
            Err(e) => match e {
                GenericError::TimeoutError => {
                    error!("Agent heartbeat timed out pinging principal");
                    Err(GenericError::TimeoutError)
                }
                e => Err(GenericError::RuntimeError(format!(
                    "Unspecified error in principal heartbeat: {}",
                    e.to_string()
                ))),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeromq::ZmqMessage;

    // TODO: add tests
}
