use async_trait::async_trait;
use cdktr_api::{API, PrincipalAPI, models::ClientResponseMessage};
use cdktr_core::exceptions::GenericError;

/// The event listener trait is for implementing components that
/// listen to external events and place onto a Queue. T refers to
/// the item that will be placed on the queue upon each event.
#[async_trait]
pub trait EventListener<T> {
    async fn start_listening(&mut self) -> Result<(), GenericError>;
    async fn run_workflow(&mut self, workflow_id: &str) -> Result<(), GenericError> {
        let api = PrincipalAPI::RunTask(workflow_id.to_string());
        let result = api.send().await;
        match result {
            Ok(r) => match r {
                ClientResponseMessage::Success => Ok(()),
                other => Err(GenericError::WorkflowError(format!(
                    "Failed to start workflow {}. Response from principal: {}",
                    workflow_id,
                    other.to_string()
                ))),
            },
            Err(e) => Err(e),
        }
    }
}
