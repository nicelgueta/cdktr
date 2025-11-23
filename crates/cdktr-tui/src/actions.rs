/// Core Action types for the flux architecture.
/// All state mutations flow through Actions dispatched to the Dispatcher.
use cdktr_core::models::RunStatus;
use std::collections::HashMap;

/// Represents all possible user intents and system events in the application
#[derive(Debug, Clone)]
pub enum Action {
    // ===== UI Actions (user-initiated) =====
    /// User switched to a different tab
    SwitchTab(TabId),

    /// User selected a workflow from the sidebar
    SelectWorkflow(String),

    /// User changed focus to a different panel
    FocusPanel(PanelId),

    /// User toggled the help overlay
    ToggleHelp,

    /// User requested to refresh workflow list
    RefreshWorkflows, // ===== Command Actions (future - placeholders) =====
    // StartWorkflow(String),
    // PauseWorkflow(String),
    // CancelWorkflow(String),
    // RetryStep(String, String),

    // ===== System/Effect Actions (emitted by Effects) =====
    /// Workflow list was successfully loaded from backend
    WorkflowListLoaded(Vec<WorkflowMetadata>),

    /// Failed to load workflow list
    WorkflowListLoadFailed(String),

    /// Workflow status was updated (future - for real-time updates)
    WorkflowStatusUpdated(String, RunStatus),

    /// Step logs were appended (future - for log streaming)
    StepLogsAppended {
        workflow_id: String,
        step_id: String,
        logs: Vec<LogLine>,
    },

    /// Generic error to display to user
    ShowError(String),

    /// Clear any displayed errors
    ClearError,

    /// Application should exit
    Quit,
}

/// Identifies different tabs in the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabId {
    Workflows,
    Admin,
}

/// Identifies different panels in the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelId {
    Sidebar,
    MainPanel,
    DetailPanel,
}

/// Metadata about a workflow returned from the backend
#[derive(Debug, Clone)]
pub struct WorkflowMetadata {
    pub task_id: String,
    pub name: String,
    pub description: String,
    pub path: String,
    // Add more fields as needed
}

impl WorkflowMetadata {
    /// Parse from the HashMap format returned by ListWorkflowStore API
    pub fn from_map(map: HashMap<String, String>) -> Self {
        Self {
            task_id: map.get("task_id").cloned().unwrap_or_default(),
            name: map.get("name").cloned().unwrap_or_default(),
            description: map.get("description").cloned().unwrap_or_default(),
            path: map.get("path").cloned().unwrap_or_default(),
        }
    }
}

/// Represents a single log line (placeholder for future log streaming)
#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: u64,
    pub level: String,
    pub message: String,
}
