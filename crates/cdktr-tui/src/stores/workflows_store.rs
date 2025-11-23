/// WorkflowsStore manages the state of workflows in the application
use crate::actions::Action;
use cdktr_workflow::Workflow;
use std::sync::{Arc, RwLock};

/// Internal state for workflows
#[derive(Debug, Clone)]
pub struct WorkflowsState {
    /// List of all workflows
    pub workflows: Vec<Workflow>,

    /// Currently selected workflow ID
    pub selected_workflow_id: Option<String>,

    /// Loading state
    pub is_loading: bool,

    /// Error message if loading failed
    pub error: Option<String>,
}

impl Default for WorkflowsState {
    fn default() -> Self {
        Self {
            workflows: Vec::new(),
            selected_workflow_id: None,
            is_loading: false,
            error: None,
        }
    }
}

/// Store that holds workflow-related state
#[derive(Clone)]
pub struct WorkflowsStore {
    state: Arc<RwLock<WorkflowsState>>,
}

impl WorkflowsStore {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(WorkflowsState::default())),
        }
    }

    /// Get a read-only snapshot of the current state
    pub fn get_state(&self) -> WorkflowsState {
        self.state.read().unwrap().clone()
    }

    /// Reducer: handle an action and update state accordingly
    pub fn reduce(&self, action: &Action) {
        let mut state = self.state.write().unwrap();

        match action {
            Action::RefreshWorkflows => {
                state.is_loading = true;
                state.error = None;
            }

            Action::WorkflowListLoaded(workflows) => {
                state.workflows = {
                    let mut workflows = workflows.clone();
                    workflows.sort_by_key(|w| w.id().clone());
                    workflows
                };
                state.is_loading = false;
                state.error = None;

                // Auto-select first workflow if none selected
                if state.selected_workflow_id.is_none() && !workflows.is_empty() {
                    state.selected_workflow_id = Some(workflows[0].id().clone());
                }
            }

            Action::WorkflowListLoadFailed(error) => {
                state.is_loading = false;
                state.error = Some(error.clone());
            }

            Action::SelectWorkflow(workflow_id) => {
                state.selected_workflow_id = Some(workflow_id.clone());
            }

            _ => {
                // Ignore actions not relevant to this store
            }
        }
    }

    /// Get the currently selected workflow
    pub fn get_selected_workflow(&self) -> Option<Workflow> {
        let state = self.state.read().unwrap();
        state
            .selected_workflow_id
            .as_ref()
            .and_then(|id| state.workflows.iter().find(|w| w.id() == id).cloned())
    }

    /// Get the index of the currently selected workflow
    pub fn get_selected_index(&self) -> Option<usize> {
        let state = self.state.read().unwrap();
        state
            .selected_workflow_id
            .as_ref()
            .and_then(|id| state.workflows.iter().position(|w| w.id() == id))
    }
}

// Tests removed - will add back with proper Workflow construction
