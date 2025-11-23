/// WorkflowsStore manages the state of workflows in the application
use crate::actions::{Action, WorkflowMetadata};
use std::sync::{Arc, RwLock};

/// Internal state for workflows
#[derive(Debug, Clone, Default)]
pub struct WorkflowsState {
    /// List of all workflows
    pub workflows: Vec<WorkflowMetadata>,

    /// Currently selected workflow ID
    pub selected_workflow_id: Option<String>,

    /// Loading state
    pub is_loading: bool,

    /// Error message if loading failed
    pub error: Option<String>,
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
                state.workflows = workflows.clone();
                state.is_loading = false;
                state.error = None;

                // Auto-select first workflow if none selected
                if state.selected_workflow_id.is_none() && !workflows.is_empty() {
                    state.selected_workflow_id = Some(workflows[0].id.clone());
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
    pub fn get_selected_workflow(&self) -> Option<WorkflowMetadata> {
        let state = self.state.read().unwrap();
        state
            .selected_workflow_id
            .as_ref()
            .and_then(|id| state.workflows.iter().find(|w| &w.id == id).cloned())
    }

    /// Get the index of the currently selected workflow
    pub fn get_selected_index(&self) -> Option<usize> {
        let state = self.state.read().unwrap();
        state
            .selected_workflow_id
            .as_ref()
            .and_then(|id| state.workflows.iter().position(|w| &w.id == id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let store = WorkflowsStore::new();
        let state = store.get_state();
        assert_eq!(state.workflows.len(), 0);
        assert_eq!(state.selected_workflow_id, None);
        assert_eq!(state.is_loading, false);
    }

    #[test]
    fn test_workflow_list_loaded() {
        let store = WorkflowsStore::new();

        let workflows = vec![
            WorkflowMetadata {
                id: "wf1".to_string(),
                name: "Workflow 1".to_string(),
                description: "Desc 1".to_string(),
                path: "/path/to/wf1".to_string(),
            },
            WorkflowMetadata {
                id: "wf2".to_string(),
                name: "Workflow 2".to_string(),
                description: "Desc 2".to_string(),
                path: "/path/to/wf2".to_string(),
            },
        ];

        store.reduce(&Action::WorkflowListLoaded(workflows));

        let state = store.get_state();
        assert_eq!(state.workflows.len(), 1);
        assert_eq!(state.selected_workflow_id, Some("wf1".to_string()));
        assert_eq!(state.is_loading, false);
    }

    #[test]
    fn test_select_workflow() {
        let store = WorkflowsStore::new();

        let workflows = vec![
            WorkflowMetadata {
                id: "wf1".to_string(),
                name: "Workflow 1".to_string(),
                description: "Desc 1".to_string(),
                path: "/path/to/wf1".to_string(),
            },
            WorkflowMetadata {
                id: "wf2".to_string(),
                name: "Workflow 2".to_string(),
                description: "Desc 2".to_string(),
                path: "/path/to/wf2".to_string(),
            },
        ];

        store.reduce(&Action::WorkflowListLoaded(workflows));
        store.reduce(&Action::SelectWorkflow("wf2".to_string()));

        let state = store.get_state();
        assert_eq!(state.selected_workflow_id, Some("wf2".to_string()));
    }
}
