mod task;

use crate::{exceptions, utils::arg_str_to_vec};
use std::collections::VecDeque;
pub use task::Task;
use zeromq::ZmqMessage;
pub mod traits;

#[derive(Debug, PartialEq)]
pub enum FlowExecutionResult {
    SUCCESS,
    CRASHED(String),
    // ABORTED(String),
    // FAILURE(String),
}

impl FlowExecutionResult {
    pub fn to_string(self) -> String {
        match self {
            Self::CRASHED(v) => v,
            _ => "".to_string(), // Self::ABORTED(v) => v,
                                 // Self::FAILURE(v) => v,
        }
    }
}

/// PubZMQMessageType enum defines the possible messages that are published
/// on the PUB wire between different components and externally. The first token
/// in a ZMQString is matched against this enum to determine whether a message
/// appears to be a supported message based on this token. It is up to the actual
/// implementation of the ZMQEncodable itself to determine whether the rest of the string
/// is valid or not for the message type.
pub enum PubZMQMessage {
    /// Standard task definition for a task without a specific executor context
    /// A message sent from the publisher like this is executed by all agents
    /// listening to the feed
    /// eg.
    /// TASKDEF|PROCESS|ls|thisdir
    TaskDef(Task),
}
impl TryFrom<ZmqMessage> for PubZMQMessage {
    type Error = exceptions::ZMQParseError;
    fn try_from(value: ZmqMessage) -> Result<Self, Self::Error> {
        let mut args: ZMQArgs = value.into();
        let msg_type = if let Some(token) = args.next() {
            token
        } else {
            return Err(exceptions::ZMQParseError::InvalidMessageType);
        };
        match msg_type.as_str() {
            "TASKDEF" => Ok(Self::TaskDef(Task::try_from(args)?)),
            _ => Err(exceptions::ZMQParseError::InvalidTaskType),
        }
    }
}

/// This struct is returned from a parsed ZMQ message after the type has
/// been determined from the first token in the message.
/// So for example, given the raw ZMQ string:
/// `TASKDEF|PROCESS|ls|thisdir`
/// The tokens would be: ["PROCESS", "ls", "thisdir"]. This is because the message
/// would have already been determined to be a task definition (TASKDEF)
pub struct ZMQArgs {
    inner: VecDeque<String>,
}

impl ZMQArgs {
    pub fn next(&mut self) -> Option<String> {
        self.inner.pop_front()
    }
    pub fn put(&mut self, item: String) {
        self.inner.push_back(item)
    }
    pub fn len(&self) -> usize {
        self.inner.len()
    }
}
impl Into<Vec<String>> for ZMQArgs {
    fn into(self) -> Vec<String> {
        self.inner.into()
    }
}

impl From<VecDeque<String>> for ZMQArgs {
    fn from(value: VecDeque<String>) -> Self {
        Self { inner: value }
    }
}

impl From<Vec<String>> for ZMQArgs {
    fn from(value: Vec<String>) -> Self {
        Self {
            inner: value.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zmq_message_type_taskdef() {
        let zmq_msg = ZmqMessage::from("TASKDEF|PROCESS|ls");
        assert!(PubZMQMessage::try_from(zmq_msg).is_ok());
    }

    #[test]
    fn zmq_message_type_invalid() {
        let zmq_msg = ZmqMessage::from("invalidinvalid");
        assert!(PubZMQMessage::try_from(zmq_msg).is_err());
    }
}
