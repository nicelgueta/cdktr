use std::{sync::Arc, time::Duration};

use crate::{
    PrincipalClient,
    models::{ClientResponseMessage, RepReqError},
};
use cdktr_core::{
    exceptions::{GenericError, ZMQParseError},
    get_cdktr_setting,
    models::ZMQArgs,
    utils::get_default_zmq_timeout,
    zmq_helpers::send_recv_with_timeout,
};

use async_trait::async_trait;
use log::{info, trace, warn};
use tokio::{sync::Mutex, time::sleep};
use zeromq::{ReqSocket, ZmqMessage};

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
}
