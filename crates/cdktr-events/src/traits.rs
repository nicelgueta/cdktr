use async_trait::async_trait;
use cdktr_api::{PrincipalAPI, models::ClientResponseMessage};
use cdktr_core::{exceptions::GenericError, get_cdktr_setting, zmq_helpers::PrincipalConnection};

/// The event listener trait is for implementing components that
/// listen to external events and send to the principal to trigger workflows
#[async_trait]
pub trait EventListener {
    async fn start_listening(&mut self) -> Result<(), GenericError>;
    async fn run_workflow(&mut self, workflow_id: &str) -> Result<(), GenericError> {
        let host = get_cdktr_setting!(CDKTR_PRINCIPAL_HOST);
        let port = get_cdktr_setting!(CDKTR_PRINCIPAL_PORT, usize);
        let connection = PrincipalConnection::new(&host, port);
        let msg = PrincipalAPI::RunTask(workflow_id.to_string()).into();
        let response = connection.request(msg).await?;
        match ClientResponseMessage::from(response) {
            ClientResponseMessage::Success => Ok(()),
            other => Err(GenericError::WorkflowError(format!(
                "Failed to start workflow {}. Response from principal: {}",
                workflow_id,
                other.to_string()
            ))),
        }
    }
}
