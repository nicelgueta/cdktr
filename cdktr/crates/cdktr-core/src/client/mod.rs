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

const RETRY_ATTEMPTS: &'static str = "10";
const RETRY_INTERVAL: u64 = 3;

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
        let max_reconnection_attempts: usize = env::var("AGENT_RECONNNECT_ATTEMPTS")
            .unwrap_or(RETRY_ATTEMPTS.to_string())
            .parse()
            .unwrap();
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
                actual_attempts += 1;
                if actual_attempts == max_reconnection_attempts {
                    error!(
                        "Max reconnect attempts reached - connection with principal has been lost"
                    );
                    return Err(GenericError::TimeoutError);
                }
                warn!(
                    " Failed to communicate to principal: {} - trying again {} seconds (attempt {} of {})",
                    reconn_result.unwrap_err().to_string(),
                    RETRY_INTERVAL,
                    actual_attempts,
                    max_reconnection_attempts
                );
                sleep(Duration::from_secs(RETRY_INTERVAL)).await;
            }
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
                trace!("Principal response: {}", msg);
                Ok(())
            }
            Err(e) => match e {
                GenericError::TimeoutError => {
                    error!("Agent heartbeat timed out pinging principal");
                    Err(GenericError::TimeoutError)
                }
                _ => panic!("Unspecified error in principal heartbeat"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeromq::ZmqMessage;

    // TODO: add tests
}
