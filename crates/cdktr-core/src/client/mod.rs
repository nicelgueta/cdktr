use log::{debug, error, info, trace, warn};
use std::{env, time::Duration};
use tokio::time::sleep;

use crate::{
    api::{PrincipalAPI, API},
    db::schema::schedules::task_name,
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
    principal_uri: String,
}

impl PrincipalClient {
    pub fn new(instance_id: String, principal_uri: String) -> Self {
        Self {
            instance_id,
            principal_uri,
        }
    }
    pub async fn register_with_principal(&mut self) -> Result<(), GenericError> {
        debug!("Registering agent with principal @ {}", &self.principal_uri);
        let max_reconnection_attempts: usize = env::var("AGENT_RECONNNECT_ATTEMPTS")
            .unwrap_or(RETRY_ATTEMPTS.to_string())
            .parse()
            .unwrap();
        let mut actual_attempts: usize = 0;
        loop {
            let request = PrincipalAPI::RegisterAgent(self.instance_id.clone());
            let reconn_result = request
                .send(&self.principal_uri, CDKTR_DEFAULT_TIMEOUT)
                .await;
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

    /// waits indefinitely for a task from the principal
    pub async fn wait_next_task(
        &self,
        sleep_interval: Duration,
        timeout: Duration,
    ) -> Result<Task, GenericError> {
        loop {
            let task_res = self.fetch_next_task(timeout).await;
            let task = match task_res {
                Ok(task_opt) => match task_opt {
                    Some(task) => task,
                    None => {
                        // no data on queue so sleep and await next call
                        sleep(sleep_interval).await;
                        continue;
                    }
                },
                Err(e) => match e {
                    GenericError::NoDataException(_err_msg) => {
                        trace!("No work on global task queue - waiting");
                        sleep(sleep_interval).await;
                        continue;
                    }
                    other_error => return Err(other_error),
                },
            };
            return Ok(task);
        }
    }

    pub async fn fetch_next_task(&self, timeout: Duration) -> Result<Option<Task>, GenericError> {
        let request = PrincipalAPI::FetchTask(self.instance_id.clone());
        match request.send(&self.principal_uri, timeout).await {
            Ok(cli_resp) => match cli_resp {
                ClientResponseMessage::Success => {
                    Err(GenericError::NoDataException("Queue empty".to_string()))
                }
                ClientResponseMessage::SuccessWithPayload(task_str) => {
                    info!("Task received from Principal -> {}", &task_str);
                    let task_res = Task::try_from(task_str);
                    return match task_res {
                        Ok(task) => Ok(Some(task)),
                        Err(e) => Err(GenericError::ZMQParseError(e)),
                    };
                }
                other => {
                    return Err(GenericError::RuntimeError(format!(
                        "Unexpected client response message received from principal: {}",
                        other.to_string()
                    )))
                }
            },
            Err(e) => match e {
                GenericError::TimeoutError => {
                    error!("Agent heartbeat timed out pinging principal");
                    return Err(GenericError::TimeoutError);
                }
                e => {
                    return Err(GenericError::RuntimeError(format!(
                        "Unspecified error in principal heartbeat: {}",
                        e.to_string()
                    )))
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {

    // TODO: add tests
}
