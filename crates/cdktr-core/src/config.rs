/// This config file lists out all the default values for the main CDKTR env configs
/// All can be overridden by either an ENV var of the same name. Some can also be overridden
/// from the command line. These should only be primitive typess
///

/// default max number of concurrent workflows an agent can handle
pub static CDKTR_AGENT_MAX_CONCURRENCY: usize = 2;

/// number of times to re-attempt a zmq request
pub static CDKTR_RETRY_ATTEMPTS: usize = 20;

/// default timeout for a zmq request
pub static CDKTR_DEFAULT_TIMEOUT_MS: usize = 3_000;

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

/// Task queue persistence interval. Used in case of failure of the principal
/// so it can pick up where it left off. Stored in APP DATA directory.
pub static CDKTR_Q_PERSISTENCE_INTERVAL_MS: usize = 1000;

/// App data directory for cdktr instances
pub static CDKTR_APP_DATA_DIRECTORY: &'static str = "$HOME/.cdktr";
