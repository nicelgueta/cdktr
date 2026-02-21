mod client;
// mod events; TODO: reinclude once the main runner is working
pub mod log_manager;
mod server;
mod taskmanager;

// public api
pub mod instance;

// some integration tests for easier debugging - skipped by default since they spawn
// indefinite loops
#[cfg(test)]
mod tests {
    use super::instance::{start_agent, start_principal};

    #[ignore]
    #[tokio::test]
    async fn test_agent() {
        start_agent("fake-instance-id".to_string(), 1).await
    }

    #[ignore]
    #[tokio::test]
    async fn test_principal() -> Result<(), cdktr_core::exceptions::GenericError> {
        start_principal(
            "0.0.0.0".to_string(),
            5561,
            "test_instance".to_string(),
            false,
        )
        .await
    }
}
