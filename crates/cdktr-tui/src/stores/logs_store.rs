/// LogsStore manages log streaming state (placeholder for future implementation)
use crate::actions::{Action, LogLine};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Internal state for logs
#[derive(Debug, Clone, Default)]
pub struct LogsState {
    /// Map of workflow_id -> step_id -> logs
    /// Placeholder structure for future log streaming
    pub logs: HashMap<String, HashMap<String, Vec<LogLine>>>,
}

/// Store that holds log-related state (placeholder)
#[derive(Clone)]
pub struct LogsStore {
    state: Arc<RwLock<LogsState>>,
}

impl LogsStore {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(LogsState::default())),
        }
    }

    /// Get a read-only snapshot of the current state
    #[allow(dead_code)]
    pub fn get_state(&self) -> LogsState {
        self.state.read().unwrap().clone()
    }

    /// Reducer: handle an action and update state accordingly
    pub fn reduce(&self, action: &Action) {
        let mut state = self.state.write().unwrap();

        match action {
            Action::StepLogsAppended {
                workflow_id,
                step_id,
                logs,
            } => {
                // Future implementation: append logs to the appropriate workflow/step
                let workflow_logs = state
                    .logs
                    .entry(workflow_id.clone())
                    .or_insert_with(HashMap::new);
                let step_logs = workflow_logs
                    .entry(step_id.clone())
                    .or_insert_with(Vec::new);
                step_logs.extend(logs.clone());
            }

            _ => {
                // Ignore actions not relevant to this store
            }
        }
    }

    /// Get logs for a specific workflow and step (placeholder)
    #[allow(dead_code)]
    pub fn get_logs(&self, workflow_id: &str, step_id: &str) -> Vec<LogLine> {
        let state = self.state.read().unwrap();
        state
            .logs
            .get(workflow_id)
            .and_then(|wf_logs| wf_logs.get(step_id))
            .cloned()
            .unwrap_or_default()
    }
}
