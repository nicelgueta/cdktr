mod connection;
pub use connection::PrincipalConnection;

use cdktr_api::{PrincipalAPI, models::ClientResponseMessage};
use cdktr_core::{
    exceptions::GenericError, get_cdktr_setting, models::RunStatus, utils::get_default_zmq_timeout,
};
use cdktr_workflow::Workflow;
use log::{debug, error, info, warn};
use std::time::Duration;
use tokio::time::sleep;
use zeromq::ZmqMessage;

/// A cloneable handle to the principal server, backed by a persistent DEALER socket.
///
/// Unlike the previous implementation that created a new REQ socket per call,
/// PrincipalClient owns one long-lived `PrincipalConnection` actor that multiplexes
/// all requests over a single DEALER socket, eliminating file-descriptor leaks.
#[derive(Clone)]
pub struct PrincipalClient {
    instance_id: String,
    connection: PrincipalConnection,
}

impl PrincipalClient {
    pub fn new(instance_id: String) -> Self {
        let host = get_cdktr_setting!(CDKTR_PRINCIPAL_HOST);
        let port = get_cdktr_setting!(CDKTR_PRINCIPAL_PORT, usize);
        Self {
            instance_id,
            connection: PrincipalConnection::new(&host, port),
        }
    }

    /// Send a ZMQ message through the persistent DEALER connection with retry on timeout.
    async fn request_with_retry(&self, msg: ZmqMessage) -> Result<ZmqMessage, GenericError> {
        let max_attempts = get_cdktr_setting!(CDKTR_RETRY_ATTEMPTS, usize);
        let delay = get_default_zmq_timeout();
        let mut attempts = 0;

        loop {
            match self.connection.request(msg.clone()).await {
                Ok(reply) => {
                    if attempts > 0 {
                        info!(
                            "Successfully reconnected with principal after {} attempt(s)",
                            attempts
                        );
                    }
                    return Ok(reply);
                }
                Err(GenericError::ZMQTimeoutError) | Err(GenericError::PrincipalTimeoutError) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        warn!(
                            "Max retry attempts ({}) reached - connection with principal has been lost",
                            max_attempts
                        );
                        return Err(GenericError::PrincipalTimeoutError);
                    }
                    warn!(
                        "Failed to communicate with principal - retrying in {} ms (attempt {} of {})",
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

    /// Register (or re-register) this agent with the principal.
    pub async fn register_with_principal(&self) -> Result<(), GenericError> {
        debug!("Registering agent with principal: {}", &self.instance_id);
        let msg: ZmqMessage = PrincipalAPI::RegisterAgent(self.instance_id.clone()).into();
        let reply = self.request_with_retry(msg).await?;
        match ClientResponseMessage::from(reply) {
            ClientResponseMessage::Success => {
                info!("Successfully registered agent with principal");
                Ok(())
            }
            other => {
                warn!("Non-success response on register: {}", other.to_string());
                Ok(())
            }
        }
    }

    /// Send a heartbeat to keep this agent registered with the principal.
    pub async fn send_heartbeat(&self) -> Result<(), GenericError> {
        let msg: ZmqMessage = PrincipalAPI::RegisterAgent(self.instance_id.clone()).into();
        match self.request_with_retry(msg).await {
            Ok(reply) => match ClientResponseMessage::from(reply) {
                ClientResponseMessage::Success => {
                    debug!("Heartbeat sent successfully");
                    Ok(())
                }
                other => {
                    warn!("Unexpected heartbeat response: {}", other.to_string());
                    Ok(())
                }
            },
            Err(e) => {
                error!("Failed to send heartbeat: {}", e);
                Err(e)
            }
        }
    }

    /// Poll the principal for the next workflow. Returns `NoDataException` when empty.
    pub async fn fetch_next_workflow(&self) -> Result<Workflow, GenericError> {
        let msg: ZmqMessage = PrincipalAPI::FetchWorkflow(self.instance_id.clone()).into();
        let reply = self.request_with_retry(msg).await?;
        match ClientResponseMessage::from(reply) {
            ClientResponseMessage::Success => {
                Err(GenericError::NoDataException("Queue empty".to_string()))
            }
            ClientResponseMessage::SuccessWithPayload(workflow_str) => {
                debug!("Workflow received from Principal -> {}", &workflow_str);
                match Workflow::try_from(workflow_str) {
                    Ok(wf) => {
                        info!("Workflow received from Principal -> {}", wf.name());
                        Ok(wf)
                    }
                    Err(e) => Err(GenericError::ParseError(format!(
                        "Failed to deserialise Workflow from principal: {}",
                        e
                    ))),
                }
            }
            other => Err(GenericError::RuntimeError(format!(
                "Unexpected response for FetchWorkflow: {}",
                other.to_string()
            ))),
        }
    }

    /// Block (polling) until a workflow is available from the principal.
    pub async fn wait_next_workflow(
        &self,
        sleep_interval: Duration,
    ) -> Result<Workflow, GenericError> {
        loop {
            match self.fetch_next_workflow().await {
                Ok(wf) => return Ok(wf),
                Err(GenericError::NoDataException(_)) => {
                    sleep(sleep_interval).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Notify the principal of a workflow status change.
    pub async fn notify_workflow_status(
        &self,
        agent_id: &str,
        workflow_id: &str,
        workflow_instance_id: &str,
        status: RunStatus,
    ) -> Result<(), GenericError> {
        let msg: ZmqMessage = PrincipalAPI::WorkflowStatusUpdate(
            agent_id.to_string(),
            workflow_id.to_string(),
            workflow_instance_id.to_string(),
            status,
        )
        .into();
        let reply = self.request_with_retry(msg).await?;
        match ClientResponseMessage::from(reply) {
            ClientResponseMessage::Success => Ok(()),
            other => {
                warn!(
                    "Unexpected response for WorkflowStatusUpdate: {}",
                    other.to_string()
                );
                Ok(())
            }
        }
    }

    /// Notify the principal of a task status change.
    pub async fn notify_task_status(
        &self,
        agent_id: &str,
        task_id: &str,
        task_execution_id: &str,
        workflow_instance_id: &str,
        status: RunStatus,
    ) -> Result<(), GenericError> {
        let msg: ZmqMessage = PrincipalAPI::TaskStatusUpdate(
            agent_id.to_string(),
            task_id.to_string(),
            task_execution_id.to_string(),
            workflow_instance_id.to_string(),
            status,
        )
        .into();
        let reply = self.request_with_retry(msg).await?;
        match ClientResponseMessage::from(reply) {
            ClientResponseMessage::Success => Ok(()),
            other => {
                warn!(
                    "Unexpected response for TaskStatusUpdate: {}",
                    other.to_string()
                );
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // TODO: integration tests requiring a live principal
}
