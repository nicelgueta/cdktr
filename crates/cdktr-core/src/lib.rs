mod api;
mod client;
mod db;
mod events;
mod exceptions;
mod executors;
mod macros;
mod models;
mod server;
mod taskmanager;
mod utils;
mod zmq_helpers;

// public api
pub mod instance;
pub mod prelude {
    pub use crate::{
        api::{APIMeta, PrincipalAPI, API},
        server::models::ClientResponseMessage,
        zmq_helpers::{get_server_tcp_uri, DEFAULT_TIMEOUT as CDKTR_DEFAULT_TIMEOUT},
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
