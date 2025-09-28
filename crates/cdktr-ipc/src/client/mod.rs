use cdktr_api::{API, PrincipalAPI, models::ClientResponseMessage};
use cdktr_core::{exceptions::GenericError, get_cdktr_setting};
use cdktr_workflow::Workflow;
use log::{debug, error, info, trace, warn};
use std::{env, time::Duration};
use tokio::time::sleep;

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
        let cdktr_retry_attempts: usize = get_cdktr_setting!(CDKTR_RETRY_ATTEMPTS, usize);
        let cdktr_default_timeout: Duration =
            Duration::from_millis(get_cdktr_setting!(CDKTR_DEFAULT_TIMEOUT_MS, usize) as u64);

        debug!("Registering agent with principal @ {}", &self.principal_uri);
        let max_reconnection_attempts = cdktr_retry_attempts;
        let mut actual_attempts: usize = 0;
        loop {
            let request = PrincipalAPI::RegisterAgent(self.instance_id.clone());
            let reconn_result = request
                .send(&self.principal_uri, cdktr_default_timeout)
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
                    return Err(GenericError::PrincipalTimeoutError);
                }
                warn!(
                    "Failed to communicate to principal: {} - trying again in {} ms (attempt {} of {})",
                    reconn_result.unwrap_err().to_string(),
                    cdktr_default_timeout.as_millis().to_string(),
                    actual_attempts,
                    max_reconnection_attempts
                );
            }
        }

        Ok(())
    }

    pub fn get_uri(&self) -> String {
        self.principal_uri.clone()
    }
    /// waits indefinitely for a workflow from the principal
    pub async fn wait_next_workflow(
        &self,
        sleep_interval: Duration,
        timeout: Duration,
    ) -> Result<Workflow, GenericError> {
        let cdktr_retry_attempts: usize = get_cdktr_setting!(CDKTR_RETRY_ATTEMPTS, usize);
        let cdktr_default_timeout: Duration =
            Duration::from_millis(get_cdktr_setting!(CDKTR_DEFAULT_TIMEOUT_MS, usize) as u64);
        let mut reconnection_attempts: usize = 0;
        loop {
            let workflow_res = self.fetch_next_workflow(timeout).await;
            let workflow = match workflow_res {
                Ok(workflow) => {
                    if reconnection_attempts > 0 {
                        info!("Successfully reconnected with principal");
                        reconnection_attempts = 0
                    };
                    workflow
                }
                Err(e) => match e {
                    GenericError::NoDataException(_err_msg) => {
                        trace!("No work on global workflow queue - waiting");
                        if reconnection_attempts > 0 {
                            info!("Successfully reconnected with principal");
                            reconnection_attempts = 0
                        };
                        sleep(sleep_interval).await;
                        continue;
                    }
                    GenericError::PrincipalTimeoutError => {
                        if reconnection_attempts == cdktr_retry_attempts {
                            error!("Max reconnection attempts reached - aborting");
                            return Err(GenericError::RuntimeError(
                                "Connection with principal host was lost. Process aborting"
                                    .to_string(),
                            ));
                        };
                        let retry_interval = cdktr_default_timeout.as_millis();
                        warn!(
                            "Failed to communicate to principal - trying again in {retry_interval} ms (attempt {reconnection_attempts} of {cdktr_retry_attempts})"
                        );
                        reconnection_attempts += 1;
                        continue;
                    }
                    other_error => return Err(other_error),
                },
            };
            return Ok(workflow);
        }
    }

    pub async fn fetch_next_workflow(&self, timeout: Duration) -> Result<Workflow, GenericError> {
        let request = PrincipalAPI::FetchWorkflow(self.instance_id.clone());
        match request.send(&self.principal_uri, timeout).await {
            Ok(cli_resp) => match cli_resp {
                ClientResponseMessage::Success => {
                    Err(GenericError::NoDataException("Queue empty".to_string()))
                }
                ClientResponseMessage::SuccessWithPayload(workflow_str) => {
                    debug!("Workflow received from Principal -> {}", &workflow_str);
                    let workflow = match Workflow::try_from(workflow_str) {
                        Ok(wf) => {
                            info!("Workflow received from Principal -> {}", wf.name());
                            wf
                        }
                        Err(e) => {
                            return Err(GenericError::ParseError(format!(
                                "Failed to read Workflow JSON from ZMQ string. Error: {}",
                                e.to_string()
                            )));
                        }
                    };
                    return Ok(workflow);
                }
                other => {
                    return Err(GenericError::RuntimeError(format!(
                        "Unexpected client response message received from principal: {}",
                        other.to_string()
                    )));
                }
            },
            Err(e) => match e {
                GenericError::ZMQTimeoutError => {
                    error!("Agent call timed out fetching workflow from principal");
                    return Err(GenericError::PrincipalTimeoutError);
                }
                e => {
                    return Err(GenericError::RuntimeError(format!(
                        "Unspecified error in principal heartbeat: {}",
                        e.to_string()
                    )));
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {

    // TODO: add tests
}
