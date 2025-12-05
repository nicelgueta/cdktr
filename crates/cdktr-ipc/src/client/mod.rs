use cdktr_api::{API, PrincipalAPI, models::ClientResponseMessage};
use cdktr_core::{exceptions::GenericError, get_cdktr_setting};
use cdktr_workflow::Workflow;
use log::{debug, error, info, trace, warn};
use std::time::Duration;
use tokio::time::sleep;

/// This client is used to house utility functions at a slightly higher level than the raw API
/// implemented by the PrincipalAPI.
#[derive(Clone)]
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

        let request = PrincipalAPI::RegisterAgent(self.instance_id.clone());
        let cli_msg = request.send_with_retry(None, None).await?;

        match cli_msg {
            ClientResponseMessage::Success => {
                info!("Successfully registered agent with principal");
                Ok(())
            }
            other => {
                warn!("Non-success message -> {}", other.to_string());
                Ok(())
            }
        }
    }

    /// Sends a heartbeat to the principal to keep this agent registered
    pub async fn send_heartbeat(&self) -> Result<(), GenericError> {
        let request = PrincipalAPI::RegisterAgent(self.instance_id.clone());
        match request.send_with_retry(None, None).await {
            Ok(ClientResponseMessage::Success) => {
                debug!("Heartbeat sent successfully");
                Ok(())
            }
            Ok(other) => {
                warn!("Unexpected heartbeat response: {}", other.to_string());
                Ok(())
            }
            Err(e) => {
                error!("Failed to send heartbeat: {}", e.to_string());
                Err(e)
            }
        }
    }

    pub fn get_uri(&self) -> String {
        self.principal_uri.clone()
    }
    /// waits indefinitely for a workflow from the principal
    pub async fn wait_next_workflow(
        &self,
        sleep_interval: Duration,
    ) -> Result<Workflow, GenericError> {
        loop {
            let workflow_res = self.fetch_next_workflow().await;
            let workflow = match workflow_res {
                Ok(workflow) => workflow,
                Err(e) => match e {
                    GenericError::NoDataException(_err_msg) => {
                        trace!("No work on global workflow queue - waiting");
                        sleep(sleep_interval).await;
                        continue;
                    }
                    other_error => return Err(other_error),
                },
            };
            return Ok(workflow);
        }
    }

    pub async fn fetch_next_workflow(&self) -> Result<Workflow, GenericError> {
        let request = PrincipalAPI::FetchWorkflow(self.instance_id.clone());
        match request.send_with_retry(None, None).await {
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
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {

    // TODO: add tests
}
