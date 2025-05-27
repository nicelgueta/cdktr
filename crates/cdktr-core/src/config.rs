use std::time::Duration;

/// number of times to re-attempt a zmq request
pub static CDKTR_RETRY_ATTEMPTS: usize = 10;

/// default timeout for a zmq request
pub static CDKTR_DEFAULT_TIMEOUT: Duration = Duration::from_millis(3000);
