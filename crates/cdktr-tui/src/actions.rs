/// Core Action types for the flux architecture.
/// All state mutations flow through Actions dispatched to the Dispatcher.
use cdktr_api::models::{AgentInfo, WorkflowStatusUpdate};
use cdktr_core::models::RunStatus;
use cdktr_ipc::log_manager::model::LogMessage;
use cdktr_workflow::Workflow;

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
    WorkflowListLoaded(Vec<Workflow>),

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

    /// Open log viewer for a workflow
    OpenLogViewer(String), // workflow_id

    /// Close the log viewer modal
    CloseLogViewer,

    /// Toggle between live tail and query mode in log viewer
    ToggleLogMode,

    /// Toggle verbose logging mode in log viewer
    ToggleVerboseLogging,

    /// A log message was received from the log stream
    LogReceived(LogMessage),

    /// Execute a log query with current time parameters
    ExecuteLogQuery,

    /// Query logs result received from backend (formatted strings)
    QueryLogsResult(Vec<String>),

    /// Query logs failed with error
    QueryLogsError(String),

    /// Principal status was updated (online/offline)
    PrincipalStatusUpdated(bool),

    /// Recent workflow status updates received
    RecentWorkflowStatusesUpdated(Vec<WorkflowStatusUpdate>),

    /// Registered agents list received from backend
    RegisteredAgentsUpdated(Vec<AgentInfo>),

    /// Scroll RunInfo panel
    ScrollRunInfo(i32), // positive = down, negative = up

    /// Update RunInfo filter input
    UpdateRunInfoFilter(String),

    /// Update Workflows filter input
    UpdateWorkflowsFilter(String),

    /// Scroll MainPanel DAG visualization
    ScrollMainPanel(i16), // positive = down, negative = up
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
    RunInfoPanel,
}

// WorkflowMetadata is now imported as Workflow from cdktr-workflow crate

/// Represents a single log line (placeholder for future log streaming)
#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: u64,
    pub level: String,
    pub message: String,
}
