/// Effects module handles side effects (I/O, network calls, etc.)
/// Effects are triggered by Actions and dispatch new Actions with results
use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::stores::{LogViewerStore, WorkflowsStore};
use cdktr_api::{PrincipalAPI, PrincipalClient, models::WorkflowStatusUpdate};
use cdktr_core::get_cdktr_setting;
use cdktr_ipc::log_manager::{client::LogsClient, model::LogMessage};
use cdktr_workflow::Workflow;
use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task;

/// Effects handler that executes side effects based on actions
pub struct Effects {
    dispatcher: Dispatcher,
    client: PrincipalClient,
    log_viewer_store: Option<LogViewerStore>,
    workflows_store: Option<WorkflowsStore>,
}

impl Effects {
    pub fn new(dispatcher: Dispatcher, client: PrincipalClient) -> Self {
        Self {
            dispatcher,
            client,
            log_viewer_store: None,
            workflows_store: None,
        }
    }

    pub fn set_log_viewer_store(&mut self, store: LogViewerStore) {
        self.log_viewer_store = Some(store);
    }

    pub fn set_workflows_store(&mut self, store: WorkflowsStore) {
        self.workflows_store = Some(store);
    }

    /// Spawn background tasks for status monitoring and workflow refresh
    pub fn spawn_background_tasks(&self) {
        self.spawn_workflow_refresh();
        self.spawn_status_monitor();
        self.spawn_workflow_status_monitor();
        self.spawn_agent_monitor();
    }

    /// Spawn a background task to monitor registered agents
    fn spawn_agent_monitor(&self) {
        let dispatcher = self.dispatcher.clone();
        let client = self.client.clone();
        let interval_ms = get_cdktr_setting!(CDKTR_TUI_STATUS_REFRESH_INTERVAL_MS, usize) as u64;

        task::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(interval_ms)).await;

                match fetch_registered_agents(&client).await {
                    Ok(agents) => {
                        dispatcher.dispatch(Action::RegisteredAgentsUpdated(agents));
                    }
                    Err(e) => {
                        log::debug!("Failed to fetch registered agents: {}", e);
                        // Don't dispatch error to avoid disrupting user experience
                    }
                }
            }
        });
    }

    /// Spawn a background task to monitor recent workflow statuses
    fn spawn_workflow_status_monitor(&self) {
        let dispatcher = self.dispatcher.clone();
        let client = self.client.clone();
        let interval_ms = get_cdktr_setting!(CDKTR_TUI_STATUS_REFRESH_INTERVAL_MS, usize) as u64;

        task::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(interval_ms)).await;

                match fetch_recent_workflow_statuses(&client).await {
                    Ok(status_updates) => {
                        dispatcher.dispatch(Action::RecentWorkflowStatusesUpdated(status_updates));
                    }
                    Err(e) => {
                        log::debug!("Failed to fetch recent workflow statuses: {}", e);
                        // Don't dispatch error to avoid disrupting user experience
                    }
                }
            }
        });
    }

    /// Spawn a background task to ping the principal and update status
    fn spawn_status_monitor(&self) {
        let dispatcher = self.dispatcher.clone();
        let client = self.client.clone();
        let interval_ms = get_cdktr_setting!(CDKTR_TUI_STATUS_REFRESH_INTERVAL_MS, usize) as u64;

        task::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(interval_ms)).await;
                let is_online = ping_principal(&client).await;
                dispatcher.dispatch(Action::PrincipalStatusUpdated(is_online));
            }
        });
    }

    /// Spawn a background task to refresh workflows periodically
    fn spawn_workflow_refresh(&self) {
        let dispatcher = self.dispatcher.clone();
        let client = self.client.clone();
        let interval_ms = get_cdktr_setting!(CDKTR_TUI_STATUS_REFRESH_INTERVAL_MS, usize) as u64;

        task::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(interval_ms)).await;

                log::debug!("Auto-refreshing workflows from principal...");
                match fetch_workflows_from_backend(&client).await {
                    Ok(workflows) => {
                        log::debug!("Auto-refresh: loaded {} workflows", workflows.len());
                        dispatcher.dispatch(Action::WorkflowListLoaded(workflows));
                    }
                    Err(e) => {
                        log::error!("Auto-refresh failed: {}", e);
                        // Don't dispatch error to avoid disrupting user experience
                    }
                }
            }
        });
    }
    /// Handle an action and execute any necessary side effects
    pub fn handle(&self, action: &Action) {
        match action {
            Action::RefreshWorkflows => {
                self.fetch_workflows();
            }
            Action::OpenLogViewer(workflow_id) => {
                self.start_log_tail(workflow_id.clone());
            }
            Action::ToggleLogMode => {
                // When switching to query mode, automatically execute a query
                self.query_logs();
            }
            Action::ExecuteLogQuery => {
                self.query_logs();
            }
            Action::ToggleVerboseLogging => {
                // When toggling verbose logging, re-execute the log query
                self.query_logs();
            }

            // Future: handle other actions that require side effects
            // Action::StartWorkflow(id) => self.start_workflow(id),
            _ => {
                // Most actions don't require side effects
            }
        }
    }

    /// Fetch workflows from the backend via ZMQ
    fn fetch_workflows(&self) {
        let dispatcher = self.dispatcher.clone();
        let client = self.client.clone();

        task::spawn(async move {
            log::info!("Fetching workflows from backend...");
            let result = fetch_workflows_from_backend(&client).await;

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

    /// Start tailing logs for a workflow
    fn start_log_tail(&self, workflow_id: String) {
        let dispatcher = self.dispatcher.clone();

        task::spawn(async move {
            log::info!("Starting log tail for workflow: {}", workflow_id);

            // Create a logs client and subscribe to the workflow
            match LogsClient::new("tui".to_string(), &workflow_id).await {
                Ok(mut client) => {
                    let (tx, mut rx) = mpsc::channel::<LogMessage>(100);

                    // Spawn listener task
                    task::spawn(async move {
                        if let Err(e) = client.listen(tx, None).await {
                            log::error!("Log listener error: {:?}", e);
                        }
                    });

                    // Forward logs to the dispatcher
                    while let Some(log_msg) = rx.recv().await {
                        dispatcher.dispatch(Action::LogReceived(log_msg));
                    }
                }
                Err(e) => {
                    log::error!("Failed to create logs client: {:?}", e);
                }
            }
        });
    }

    /// Query logs from the backend based on time range
    fn query_logs(&self) {
        let dispatcher = self.dispatcher.clone();
        let client = self.client.clone();

        // Get time range and workflow_id from log viewer store
        let (start_ts, end_ts, workflow_id, verbose) =
            if let Some(ref store) = self.log_viewer_store {
                let state = store.get_state();
                let start_ts = state.start_time.timestamp_millis() as u64;
                let end_ts = state.end_time.timestamp_millis() as u64;
                (start_ts, end_ts, state.workflow_id.clone(), state.verbose)
            } else {
                // Fallback to default if store not set
                let end_time = Utc::now();
                let start_time = end_time - chrono::Duration::days(2);
                (
                    start_time.timestamp_millis() as u64,
                    end_time.timestamp_millis() as u64,
                    None,
                    false,
                )
            };

        task::spawn(async move {
            log::info!("Querying logs from {} to {}", start_ts, end_ts);

            match query_logs_from_backend(&client, start_ts, end_ts, workflow_id, verbose).await {
                Ok(logs) => {
                    log::info!("Successfully queried {} log entries", logs.len());
                    dispatcher.dispatch(Action::QueryLogsResult(logs));
                }
                Err(e) => {
                    log::error!("Failed to query logs: {}", e);
                    dispatcher.dispatch(Action::QueryLogsError(e));
                }
            }
        });
    }
}

/// Query logs from the backend (ZMQ call to PrincipalAPI)
async fn query_logs_from_backend(
    client: &PrincipalClient,
    start_ts: u64,
    end_ts: u64,
    workflow_id: Option<String>,
    verbose: bool,
) -> Result<Vec<String>, String> {
    let api_msg = PrincipalAPI::QueryLogs(
        Some(end_ts),
        Some(start_ts),
        workflow_id, // Use the workflow_id from the viewer
        None,        // workflow_instance_id
        verbose,     // verbose
    );

    match client.send(api_msg).await {
        Ok(response) => {
            let payload = response.payload();
            log::debug!("Got log query payload: {} bytes", payload.len());

            match serde_json::from_str::<Vec<String>>(&payload) {
                Ok(logs) => Ok(logs),
                Err(e) => Err(format!("Failed to parse log data: {}", e)),
            }
        }
        Err(e) => Err(format!("ZMQ request failed: {}", e)),
    }
}

/// Fetch workflows from the backend (ZMQ call to PrincipalAPI)
async fn fetch_workflows_from_backend(client: &PrincipalClient) -> Result<Vec<Workflow>, String> {
    let api_msg = PrincipalAPI::ListWorkflowStore;

    match client.send(api_msg).await {
        Ok(response) => {
            let payload = response.payload();

            log::debug!("Got payload from backend: {:?}", payload);

            let parsed_payload = serde_json::from_str::<HashMap<String, Workflow>>(&payload);

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

/// Ping the principal to check if it's online
async fn ping_principal(client: &PrincipalClient) -> bool {
    let api_msg = PrincipalAPI::Ping;
    match client.send(api_msg).await {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Fetch the latest status updates for recent workflows
async fn fetch_recent_workflow_statuses(
    client: &PrincipalClient,
) -> Result<Vec<WorkflowStatusUpdate>, String> {
    let api_msg = PrincipalAPI::GetRecentWorkflowStatuses;
    match client.send(api_msg).await {
        Ok(response) => {
            let payload = response.payload();

            match serde_json::from_str::<Vec<WorkflowStatusUpdate>>(&payload) {
                Ok(status_updates) => Ok(status_updates),
                Err(e) => Err(format!("Failed to parse status updates: {}", e)),
            }
        }
        Err(e) => Err(format!("ZMQ request failed: {}", e)),
    }
}

/// Fetch the list of registered agents
async fn fetch_registered_agents(
    client: &PrincipalClient,
) -> Result<Vec<cdktr_api::models::AgentInfo>, String> {
    let api_msg = PrincipalAPI::GetRegisteredAgents;
    match client.send(api_msg).await {
        Ok(response) => {
            let payload = response.payload();

            match serde_json::from_str::<Vec<cdktr_api::models::AgentInfo>>(&payload) {
                Ok(agents) => Ok(agents),
                Err(e) => Err(format!("Failed to parse agent info: {}", e)),
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
