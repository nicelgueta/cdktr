use crate::{API, PrincipalAPI, models::ClientResponseMessage};
use cdktr_core::{
    exceptions::{GenericError, ZMQParseError},
    get_cdktr_setting,
    utils::{get_default_zmq_timeout, get_principal_uri},
    zmq_helpers::{get_zmq_req, send_recv_with_timeout},
};
use cdktr_workflow::Workflow;
use log::{debug, error, info, trace, warn};
use std::{sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::sleep};
use zeromq::ReqSocket;

/// This client is used to house utility functions at a slightly higher level than the raw API
/// implemented by the PrincipalAPI.
#[derive(Clone)]
pub struct PrincipalClient {
    /// ID of the principal currently subscribed to
    instance_id: String,
    req_socket: Arc<Mutex<ReqSocket>>,
}

impl PrincipalClient {
    pub async fn new(instance_id: String) -> Result<Self, GenericError> {
        Ok(Self {
            instance_id,
            req_socket: Arc::new(Mutex::new(get_zmq_req(&get_principal_uri()).await?)),
        })
    }

    pub async fn send(&self, msg: PrincipalAPI) -> Result<ClientResponseMessage, GenericError> {
        let zmq_reponse_message = send_recv_with_timeout(
            self.req_socket.clone(),
            msg.into(),
            get_default_zmq_timeout(),
        )
        .await?;
        Ok(ClientResponseMessage::from(zmq_reponse_message))
    }

    /// Send a message with retry logic for PrincipalTimeoutError
    ///
    /// This method will retry sending the message up to max_retries times if a
    /// PrincipalTimeoutError occurs. Other errors are returned immediately.
    ///
    /// # Arguments
    /// * `tcp_uri` - The URI to send the message to
    /// * `timeout` - Timeout for each individual send attempt
    /// * `max_retries` - Maximum number of retry attempts (defaults to CDKTR_RETRY_ATTEMPTS if None)
    /// * `retry_delay` - Delay between retry attempts (defaults to timeout if None)
    async fn send_with_retry(
        self,
        msg: PrincipalAPI,
        max_retries: Option<usize>,
        retry_delay: Option<Duration>,
    ) -> Result<ClientResponseMessage, GenericError>
    where
        Self: Sized + Clone,
    {
        let default_timeout = get_default_zmq_timeout();
        let max_attempts =
            max_retries.unwrap_or_else(|| get_cdktr_setting!(CDKTR_RETRY_ATTEMPTS, usize));
        let delay = retry_delay.unwrap_or(default_timeout);
        let mut attempts = 0;

        loop {
            let result = self.send(msg.clone()).await;

            match result {
                Ok(response) => {
                    if attempts > 0 {
                        info!(
                            "Successfully re-connected with principal after {} attempt(s)",
                            attempts
                        );
                    }
                    return Ok(response);
                }
                Err(GenericError::PrincipalTimeoutError) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        warn!(
                            "Max retry attempts ({}) reached - connection with principal has been lost",
                            max_attempts
                        );
                        return Err(GenericError::PrincipalTimeoutError);
                    }
                    warn!(
                        "Failed to communicate to principal - trying again in {} ms (attempt {} of {})",
                        delay.as_millis(),
                        attempts,
                        max_attempts
                    );
                    sleep(delay).await;
                }
                Err(GenericError::ZMQParseError(ZMQParseError::ParseError(ref msg)))
                    if msg.contains("Connection reset by peer") || msg.contains("Codec Error") =>
                {
                    attempts += 1;
                    if attempts >= max_attempts {
                        warn!(
                            "Max retry attempts ({}) reached - connection with principal has been lost",
                            max_attempts
                        );
                        return Err(GenericError::ZMQParseError(ZMQParseError::ParseError(
                            msg.clone(),
                        )));
                    }
                    warn!(
                        "Connection error ({}), trying again in {} ms (attempt {} of {})",
                        msg,
                        delay.as_millis(),
                        attempts,
                        max_attempts
                    );
                    sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub async fn register_with_principal(&mut self) -> Result<(), GenericError> {
        debug!(
            "Registering agent with principal with {}",
            &self.instance_id
        );

        let request = PrincipalAPI::RegisterAgent(self.instance_id.clone());
        let cli_msg = self.send(request).await?;

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
        match self.send(request).await {
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
        match self.send(request).await {
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
