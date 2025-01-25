use log::{debug, error, info, trace, warn};
use std::{env, time::Duration};
use tokio::time::sleep;

use crate::{
    api::{PrincipalAPI, API},
    exceptions::GenericError,
    prelude::ClientResponseMessage,
    prelude::CDKTR_DEFAULT_TIMEOUT,
};

mod helpers;

/// This client is used to house utility functions at a slightly higher level than the raw API
/// implemented by the PrincipalAPI.
pub struct PrincipalClient {
    /// ID of the principal currently subscribed to
    instance_id: String,
}

impl PrincipalClient {
    pub fn new(instance_id: String) -> Self {
        Self { instance_id }
    }
    pub async fn register_with_principal(&self, principal_uri: &str) -> Result<(), GenericError> {
        debug!("Registering agent with principal @ {}", &principal_uri);
        let max_reconnection_attempts = env::var("AGENT_RECONNNECT_ATTEMPTS")
            .unwrap_or("5".to_string())
            .parse::<usize>()
            .unwrap_or({
                warn!("Env var AGENT_RECONNNECT_ATTEMPTS specified but is not a valid number - using default 5");
                5
            });
        let mut actual_attempts: usize = 0;
        loop {
            let request = PrincipalAPI::RegisterAgent(self.instance_id.clone());
            let reconn_result = request.send(principal_uri, CDKTR_DEFAULT_TIMEOUT).await;
            if let Ok(cli_msg) = reconn_result {
                match cli_msg {
                    ClientResponseMessage::Success => {
                        info!("Successfully registered agent with principal");
                        break;
                    }
                    other => {
                        warn!("Non-success message -> {}", {
                            let m: String = other.into();
                            m
                        });
                    }
                }
            } else {
                warn!(
                    "Failed to communicate to principal: {}",
                    reconn_result.unwrap_err().to_string()
                )
            };
            actual_attempts += 1;
            if actual_attempts == max_reconnection_attempts {
                error!("Max reconnect attempts reached - exiting");
                return Err(GenericError::TimeoutError);
            }
            warn!("Unable to reconnect to principal - trying again 5 seconds");
            sleep(Duration::from_secs(5)).await;
        }

        Ok(())
    }

    pub async fn heartbeat(
        &self,
        principal_uri: &str,
        timeout: Duration,
    ) -> Result<(), GenericError> {
        let request = PrincipalAPI::Ping;
        match request.send(&principal_uri, timeout).await {
            Ok(cli_resp) => {
                let msg: String = cli_resp.into();
                trace!("Principal response: {}", msg)
            }
            Err(e) => match e {
                GenericError::TimeoutError => {
                    error!("Agent heartbeat timed out pinging principal");
                }
                _ => panic!("Unspecified error in principal heartbeat"),
            },
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeromq::ZmqMessage;

    // TODO: add tests
}
