use crate::{PrincipalAPI, models::ClientResponseMessage};
use cdktr_core::{
    exceptions::{GenericError, ZMQParseError},
    get_cdktr_setting,
    utils::{get_default_zmq_timeout, get_principal_uri},
    zmq_helpers::{get_zmq_req, get_zmq_req_with_timeout, send_recv_with_timeout},
};
use cdktr_workflow::Workflow;
use log::{debug, error, info, trace, warn};
use std::sync::atomic::{AtomicU64, Ordering};
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
    /// Lock to ensure only one reconnection attempt happens at a time
    reconnect_lock: Arc<Mutex<()>>,
    /// Version counter that increments with each successful reconnection
    /// Allows waiting coroutines to detect if reconnection already happened
    connection_version: Arc<AtomicU64>,
}

impl PrincipalClient {
    pub async fn new(instance_id: String) -> Result<Self, GenericError> {
        Ok(Self {
            instance_id,
            req_socket: Arc::new(Mutex::new(get_zmq_req(&get_principal_uri()).await?)),
            reconnect_lock: Arc::new(Mutex::new(())),
            connection_version: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Recreates the REQ socket connection to the principal
    ///
    /// This method includes retry logic with timeout. Only one coroutine will attempt
    /// reconnection at a time - others will wait for that attempt to complete.
    /// If another coroutine already successfully reconnected (indicated by connection_version
    /// change), this method will return immediately without attempting reconnection.
    /// If the reconnection fails after max retries, returns an error.
    pub async fn reconnect(
        &self,
        error_version: u64,
        attempts: usize,
        retry_delay: Duration,
    ) -> Result<(), GenericError> {
        // Acquire the reconnect lock - only one coroutine reconnects at a time
        let _reconnect_guard = self.reconnect_lock.lock().await;

        // Check if someone already reconnected while we were waiting
        let current_version = self.connection_version.load(Ordering::SeqCst);
        if current_version > error_version {
            info!("Socket already reconnected by another coroutine, continuing...");
            return Ok(());
        }

        info!("Attempting to recreate REQ socket connection to principal");

        let max_attempts = attempts;
        let principal_uri = get_principal_uri();

        for attempt in 1..=max_attempts {
            match get_zmq_req_with_timeout(&principal_uri, retry_delay).await {
                Ok(new_socket) => {
                    let mut socket_guard = self.req_socket.lock().await;
                    *socket_guard = new_socket;
                    // Increment version to signal successful reconnection
                    self.connection_version.fetch_add(1, Ordering::SeqCst);
                    info!(
                        "REQ socket successfully recreated after {} attempt(s)",
                        attempt
                    );
                    return Ok(());
                }
                Err(e) => {
                    if attempt >= max_attempts {
                        error!(
                            "Failed to reconnect to principal after {} attempts. Giving up.",
                            max_attempts
                        );
                        return Err(GenericError::ZMQParseError(ZMQParseError::ParseError(
                            format!("Unable to reconnect to principal: {}", e.to_string()),
                        )));
                    }
                    warn!(
                        "Reconnection attempt {} of {} failed: {}. Retrying in {} ms...",
                        attempt,
                        max_attempts,
                        e.to_string(),
                        retry_delay.as_millis()
                    );
                    sleep(retry_delay).await;
                }
            }
        }

        Err(GenericError::ZMQParseError(ZMQParseError::ParseError(
            "Reconnection failed after all attempts".to_string(),
        )))
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
    /// * `msg` - The PrincipalAPI message to send
    /// * `max_retries` - Maximum number of retry attempts (defaults to CDKTR_RETRY_ATTEMPTS if None)
    /// * `retry_delay` - Delay between retry attempts (defaults to timeout if None)
    pub async fn send_with_retry(
        &self,
        msg: PrincipalAPI,
        max_retries: Option<usize>,
        retry_delay: Option<Duration>,
    ) -> Result<ClientResponseMessage, GenericError>
    where
        Self: Sized + Clone,
    {
        let max_attempts =
            max_retries.unwrap_or_else(|| get_cdktr_setting!(CDKTR_RETRY_ATTEMPTS, usize));

        let retry_delay = retry_delay.unwrap_or_else(|| get_default_zmq_timeout());

        loop {
            let result = self.send(msg.clone()).await;

            match result {
                Ok(response) => {
                    return Ok(response);
                }
                Err(GenericError::PrincipalTimeoutError) => {
                    warn!("Connection timeout detected, attempting to reconnect...");
                    // Capture the current connection version before attempting reconnection
                    let error_version = self.connection_version.load(Ordering::SeqCst);
                    // Reconnect will handle all retries internally
                    // If another coroutine already reconnected, this will return immediately
                    self.reconnect(error_version, max_attempts, retry_delay)
                        .await?;
                }
                Err(GenericError::ZMQParseError(ZMQParseError::ParseError(ref msg)))
                    if msg.contains("Connection reset by peer")
                        || msg.contains("Codec Error")
                        || msg.contains("Broken pipe")
                        || msg.contains("No message received")
                        || msg.contains("Connection refused")
                        || msg.contains("Unable to send message") =>
                {
                    warn!(
                        "Connection error detected: {}, attempting to reconnect...",
                        msg
                    );
                    // Capture the current connection version before attempting reconnection
                    let error_version = self.connection_version.load(Ordering::SeqCst);
                    // Reconnect will handle all retries internally
                    // If another coroutine already reconnected, this will return immediately
                    self.reconnect(error_version, max_attempts, retry_delay)
                        .await?;
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
        // Use retry logic to keep trying until max attempts reached
        let cli_msg = self.send_with_retry(request, None, None).await?;

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
        // Use retry logic to keep trying to connect to principal
        match self.send_with_retry(request, None, None).await {
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
