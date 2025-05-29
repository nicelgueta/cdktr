/// This config file lists out all the default values for the main CDKTR env configs
/// All can be overridden by either an ENV var of the same name. Some can also be overridden
/// from the command line. These should only be primitive typess
///

/// default max number of concurrent tasks an agent can handle
pub static CDKTR_AGENT_MAX_CONCURRENCY: usize = 10;

/// number of times to re-attempt a zmq request
pub static CDKTR_RETRY_ATTEMPTS: usize = 10;

/// default timeout for a zmq request
pub static CDKTR_DEFAULT_TIMEOUT_MS: usize = 3000;

/// hostname of the principal instance
pub static CDKTR_PRINCIPAL_HOST: &'static str = "0.0.0.0";

/// default port of the principal instance
pub static CDKTR_PRINCIPAL_PORT: usize = 5561;

/// Default workflow directory
pub static CDKTR_WORKFLOW_DIR: &'static str = "workflows";
