mod api;
mod client;
// mod events; TODO: reinclude once the main runner is working
mod server;
mod taskmanager;

// public api
pub mod instance;
pub mod prelude {
    pub use crate::{
        api::{APIMeta, PrincipalAPI, API},
        server::models::ClientResponseMessage,
    };
}

// some integration tests for easier debugging - skipped by default since they spawn
// indefinite loops
#[cfg(test)]
mod tests {
    use super::instance::start_agent;

    #[ignore]
    #[tokio::test]
    async fn test_agent() {
        start_agent(
            "fake-instance-id".to_string(),
            "0.0.0.0".to_string(),
            5561,
            1,
        )
        .await
    }
}
