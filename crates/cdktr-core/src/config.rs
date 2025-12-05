/// This config file lists out all the default values for the main CDKTR env configs
/// All can be overridden by either an ENV var of the same name. Some can also be overridden
/// from the command line. These should only be primitive types
///
///

/// default log level
pub static CDKTR_LOG_LEVEL: &'static str = "INFO";

/// default max number of concurrent workflows an agent can handle
pub static CDKTR_AGENT_MAX_CONCURRENCY: usize = 5;

/// number of times to re-attempt a zmq request
pub static CDKTR_RETRY_ATTEMPTS: usize = 20;

/// default timeout for a zmq request
pub static CDKTR_DEFAULT_ZMQ_TIMEOUT_MS: usize = 3_000;

/// hostname of the principal instance
pub static CDKTR_PRINCIPAL_HOST: &'static str = "0.0.0.0";

/// default port of the principal instance
pub static CDKTR_PRINCIPAL_PORT: usize = 5561;

/// listening port for the principal log manager
pub static CDKTR_LOGS_LISTENING_PORT: usize = 5562;

/// publishing port for the principal log manager
pub static CDKTR_LOGS_PUBLISHING_PORT: usize = 5563;

/// Default workflow directory
pub static CDKTR_WORKFLOW_DIR: &'static str = "workflows";

/// Interval to refresh the workflow directory with any changes
/// useful for CICD environments where new workflows are added without
/// having to bounce any services
pub static CDKTR_WORKFLOW_DIR_REFRESH_FREQUENCY_S: usize = 60;

/// (low-level config) Interval at which the Scheduler should check whether
/// a workflow is ready to start
pub static CDKTR_SCHEDULER_START_POLL_FREQUENCY_MS: usize = 500;

/// Task queue persistence interval. Used in case of failure of the principal
/// so it can pick up where it left off. Stored in APP DATA directory.
pub static CDKTR_Q_PERSISTENCE_INTERVAL_MS: usize = 1000;

/// App data directory for cdktr instances
pub static CDKTR_APP_DATA_DIRECTORY: &'static str = "$HOME/.cdktr";

/// Path to the location of the main database for the principal instance
pub static CDKTR_DB_PATH: &'static str = "$HOME/.cdktr/app.db";

/// TUI refresh interval for principal status checks (in milliseconds)
pub static CDKTR_TUI_STATUS_REFRESH_INTERVAL_MS: usize = 1000;

/// Agent heartbeat timeout in milliseconds. If an agent hasn't sent a heartbeat
/// within this duration, any running workflows will be marked as CRASHED
pub static CDKTR_AGENT_HEARTBEAT_TIMEOUT_MS: usize = 30_000;
