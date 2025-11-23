pub mod app_logs_store;
pub mod log_viewer_store;
pub mod logs_store;
pub mod ui_store;
/// Store modules that hold application state
/// Each store is responsible for a slice of the application state
pub mod workflows_store;

pub use app_logs_store::AppLogsStore;
pub use log_viewer_store::LogViewerStore;
pub use logs_store::LogsStore;
pub use ui_store::UIStore;
pub use workflows_store::WorkflowsStore;
