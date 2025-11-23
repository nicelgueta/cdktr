/// Effects module handles side effects (I/O, network calls, etc.)
/// Effects are triggered by Actions and dispatch new Actions with results
use crate::actions::{Action, WorkflowMetadata};
use crate::dispatcher::Dispatcher;
use cdktr_api::{API, PrincipalAPI};
use cdktr_core::{get_cdktr_setting, zmq_helpers::get_server_tcp_uri};
use std::collections::HashMap;
use std::time::Duration;
use tokio::task;

/// Effects handler that executes side effects based on actions
pub struct Effects {
    dispatcher: Dispatcher,
}

impl Effects {
    pub fn new(dispatcher: Dispatcher) -> Self {
        Self { dispatcher }
    }

    /// Handle an action and execute any necessary side effects
    pub fn handle(&self, action: &Action) {
        match action {
            Action::RefreshWorkflows => {
                self.fetch_workflows();
            }

            // Future: handle other actions that require side effects
            // Action::StartWorkflow(id) => self.start_workflow(id),
            // Action::FetchLogs(workflow_id, step_id) => self.fetch_logs(workflow_id, step_id),
            _ => {
                // Most actions don't require side effects
            }
        }
    }

    /// Fetch workflows from the backend via ZMQ
    fn fetch_workflows(&self) {
        let dispatcher = self.dispatcher.clone();

        task::spawn(async move {
            log::info!("Fetching workflows from backend...");
            let result = fetch_workflows_from_backend().await;

            match result {
                Ok(workflows) => {
                    log::info!("Successfully loaded {} workflows", workflows.len());
                    dispatcher.dispatch(Action::WorkflowListLoaded(workflows));
                }
                Err(e) => {
                    log::error!("Failed to load workflows: {}", e);
                    dispatcher.dispatch(Action::WorkflowListLoadFailed(e));
                }
            }
        });
    }
}

/// Fetch workflows from the backend (ZMQ call to PrincipalAPI)
async fn fetch_workflows_from_backend() -> Result<Vec<WorkflowMetadata>, String> {
    let api_msg = PrincipalAPI::ListWorkflowStore;

    let uri = get_server_tcp_uri(
        &get_cdktr_setting!(CDKTR_PRINCIPAL_HOST),
        get_cdktr_setting!(CDKTR_PRINCIPAL_PORT, usize),
    );

    let timeout = Duration::from_millis(get_cdktr_setting!(CDKTR_DEFAULT_TIMEOUT_MS, usize) as u64);

    match api_msg.send(&uri, timeout).await {
        Ok(response) => {
            let payload = response.payload();

            log::debug!("Got payload from backend: {:?}", payload);

            let parsed_payload =
                serde_json::from_str::<HashMap<String, WorkflowMetadata>>(&payload);

            // Parse JSON response
            match parsed_payload {
                Ok(workflows) => {
                    let workflow_list = workflows.into_values().collect();
                    Ok(workflow_list)
                }
                Err(e) => Err(format!("Failed to parse workflow data: {}", e)),
            }
        }
        Err(e) => Err(format!("ZMQ request failed: {}", e)),
    }
}

// Placeholder for future log fetching effect
// async fn fetch_logs_from_backend(workflow_id: &str, step_id: &str) -> Result<Vec<LogLine>, String> {
//     // TODO: Implement using PrincipalAPI::QueryLogs
//     Ok(Vec::new())
// }
