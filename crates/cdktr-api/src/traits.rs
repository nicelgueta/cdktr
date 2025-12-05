use std::time::Duration;

use crate::models::{ClientResponseMessage, RepReqError};
use cdktr_core::{
    exceptions::{GenericError, ZMQParseError},
    get_cdktr_setting,
    models::ZMQArgs,
    utils::get_default_zmq_timeout,
    zmq_helpers::send_recv_with_timeout,
};

use async_trait::async_trait;
use log::{trace, warn};
use tokio::time::sleep;
use zeromq::ZmqMessage;

pub struct APIMeta {
    action: String,
    #[allow(dead_code)]
    description: String,
}
impl APIMeta {
    pub const fn new(action: String, description: String) -> Self {
        Self {
            action,
            description,
        }
    }
    pub fn try_to_api<T>(&self) -> Result<T, RepReqError>
    where
        T: API,
        <T as TryFrom<String>>::Error: Into<RepReqError>,
    {
        T::try_from(self.action.clone()).map_err(Into::into)
    }
}

/// The API trait defines the interface for the ZMQ-based APIs that external modules and systems
/// can leverage to communicate with CDKTR. The APIs are also used internally between different components
///
#[async_trait]
pub trait API: Into<ZmqMessage> + TryFrom<ZmqMessage> + TryFrom<String> + TryFrom<ZMQArgs> {
    // returns the metadata for all implemented endpoints
    fn get_meta(&self) -> Vec<APIMeta>;

    /// Convert the message to a string to pass on ZMQ
    fn to_string(&self) -> String;

    fn get_tcp_uri(&self) -> String;

    /// Default implementation for sending the message to a destination REP socket
    async fn send(self) -> Result<ClientResponseMessage, GenericError> {
        let tcp_uri = self.get_tcp_uri();
        trace!("Requesting @ {} with msg: {}", tcp_uri, self.to_string());
        let timeout = get_default_zmq_timeout();
        let zmq_m = send_recv_with_timeout(tcp_uri.to_string(), self.into(), timeout)
            .await
            .map_err(|e| {
                if let GenericError::ZMQTimeoutError = e {
                    GenericError::PrincipalTimeoutError
                } else {
                    e
                }
            })?;
        // dbg!(&zmq_m);
        let cli_msg = ClientResponseMessage::from(zmq_m);
        // dbg!(&cli_msg);
        Ok(cli_msg)
    }

    /// Send a message with retry logic for PrincipalTimeoutError
    ///
    /// This method will retry sending the message up to max_retries times if a
    /// PrincipalTimeoutError occurs. Other errors are returned immediately.
    ///
    /// # Arguments
    /// * `tcp_uri` - The URI to send the message to
    /// * `timeout` - Timeout for each individual send attempt
    /// * `max_retries` - Maximum number of retry attempts (defaults to CDKTR_RETRY_ATTEMPTS if None)
    /// * `retry_delay` - Delay between retry attempts (defaults to timeout if None)
    async fn send_with_retry(
        self,
        max_retries: Option<usize>,
        retry_delay: Option<Duration>,
    ) -> Result<ClientResponseMessage, GenericError>
    where
        Self: Sized + Clone,
    {
        let max_attempts =
            max_retries.unwrap_or_else(|| get_cdktr_setting!(CDKTR_RETRY_ATTEMPTS, usize));
        let delay = retry_delay.unwrap_or(get_default_zmq_timeout());
        let mut attempts = 0;

        loop {
            let result = self.clone().send().await;

            match result {
                Ok(response) => return Ok(response),
                Err(GenericError::PrincipalTimeoutError) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        warn!(
                            "Max retry attempts ({}) reached - connection with principal has been lost",
                            max_attempts
                        );
                        return Err(GenericError::PrincipalTimeoutError);
                    }
                    warn!(
                        "Failed to communicate to principal - trying again in {} ms (attempt {} of {})",
                        delay.as_millis(),
                        attempts,
                        max_attempts
                    );
                    sleep(delay).await;
                }
                Err(GenericError::ZMQParseError(ZMQParseError::ParseError(ref msg)))
                    if msg.contains("Connection reset by peer") || msg.contains("Codec Error") =>
                {
                    attempts += 1;
                    if attempts >= max_attempts {
                        warn!(
                            "Max retry attempts ({}) reached - connection with principal has been lost",
                            max_attempts
                        );
                        return Err(GenericError::ZMQParseError(ZMQParseError::ParseError(
                            msg.clone(),
                        )));
                    }
                    warn!(
                        "Connection error ({}), trying again in {} ms (attempt {} of {})",
                        msg,
                        delay.as_millis(),
                        attempts,
                        max_attempts
                    );
                    sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}
