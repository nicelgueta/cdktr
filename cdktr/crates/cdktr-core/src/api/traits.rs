use std::time::Duration;

use crate::{
    exceptions::GenericError,
    models::ZMQArgs,
    server::models::{ClientResponseMessage, RepReqError},
    zmq_helpers::send_recv_with_timeout,
};

use async_trait::async_trait;
use log::trace;
use zeromq::ZmqMessage;

pub struct APIMeta {
    action: String,
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

    /// Default implementation for sending the message to a destination REP socket
    async fn send(
        self,
        tcp_uri: &str,
        timeout: Duration,
    ) -> Result<ClientResponseMessage, GenericError> {
        trace!("Requesting @ {} with msg: {}", tcp_uri, self.to_string());
        let zmq_m = send_recv_with_timeout(tcp_uri.to_string(), self.into(), timeout).await?;
        Ok(ClientResponseMessage::from(zmq_m))
    }
}
