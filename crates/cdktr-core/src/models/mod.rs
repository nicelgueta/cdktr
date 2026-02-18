use crate::{
    exceptions,
    utils::{arg_str_to_vecd, vecd_to_arg_str},
};
use bytes::Bytes;
use std::collections::VecDeque;
use zeromq::ZmqMessage;
pub mod traits;

#[derive(Debug, PartialEq)]
pub enum FlowExecutionResult {
    SUCCESS,
    CRASHED(String),
    FAILURE(String),
    // ABORTED(String),
}

impl FlowExecutionResult {
    pub fn _to_string(self) -> String {
        match self {
            Self::CRASHED(v) => v,
            Self::FAILURE(v) => v,
            _ => "".to_string(), // Self::ABORTED(v) => v,
                                 // Self::FAILURE(v) => v,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum RunType {
    Workflow,
    Task,
}
impl RunType {
    pub fn to_string(&self) -> String {
        match self {
            Self::Workflow => "Workflow".to_string(),
            Self::Task => "Task".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum RunStatus {
    PENDING,
    RUNNING,
    WAITING,
    COMPLETED,
    FAILED,
    CRASHED,
}
impl TryFrom<String> for RunStatus {
    type Error = exceptions::GenericError;
    fn try_from(value: String) -> Result<Self, exceptions::GenericError> {
        match value.as_str() {
            "PENDING" => Ok(RunStatus::PENDING),
            "RUNNING" => Ok(RunStatus::RUNNING),
            "WAITING" => Ok(RunStatus::WAITING),
            "COMPLETED" => Ok(RunStatus::COMPLETED),
            "FAILED" => Ok(RunStatus::FAILED),
            "CRASHED" => Ok(RunStatus::CRASHED),
            _ => Err(exceptions::GenericError::ParseError(format!(
                "Unrecognised task status: {}",
                value
            ))),
        }
    }
}
impl RunStatus {
    pub fn to_string(&self) -> String {
        match self {
            RunStatus::PENDING => String::from("PENDING"),
            RunStatus::RUNNING => String::from("RUNNING"),
            RunStatus::WAITING => String::from("WAITING"),
            RunStatus::COMPLETED => String::from("COMPLETED"),
            RunStatus::FAILED => String::from("FAILED"),
            RunStatus::CRASHED => String::from("CRASHED"),
        }
    }
}

/// The ZMQArgs struct acts as an iterator of arguments that other
/// functions and structs can use to iterate over the pipe-delimited
/// messages sent over ZMQ. To avoid clashing with pipes, the \ character
/// is used as an escape. Any intended \ character should be doubled \\ in
/// order to avoid potential parsing issues.
#[derive(Debug, Clone)]
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
    pub fn to_string(&self) -> String {
        self.clone().into()
    }
}
impl Into<Vec<String>> for ZMQArgs {
    fn into(self) -> Vec<String> {
        self.inner.into()
    }
}
impl Into<String> for ZMQArgs {
    fn into(self) -> String {
        vecd_to_arg_str(&self.inner)
    }
}

/// creating ZMQArgs from string automatically escapes pipes
impl From<String> for ZMQArgs {
    fn from(value: String) -> Self {
        Self {
            inner: arg_str_to_vecd(&value),
        }
    }
}

impl From<Bytes> for ZMQArgs {
    fn from(value: Bytes) -> Self {
        let s = String::from_utf8_lossy(&value).to_string();
        Self::from(s)
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

impl Into<ZMQArgs> for ZmqMessage {
    fn into(self) -> ZMQArgs {
        let raw_msg = String::try_from(self);
        let raw_string = match raw_msg {
            Ok(s) => s,
            Err(e_str) => e_str.to_string(),
        };
        ZMQArgs::from(raw_string)
    }
}

/// Agent metadata held by principal that is used by the task router
/// to decide which agent to route tasks to and by the server to determine
/// status
#[derive(Clone, Debug)]
pub struct AgentMeta {
    agent_id: String,
    running_tasks: usize,
    pub last_ping_timestamp: i64,
}
impl AgentMeta {
    pub fn new(agent_id: String, last_ping_timestamp: i64) -> Self {
        Self {
            agent_id,
            last_ping_timestamp,
            running_tasks: 0,
        }
    }
    pub fn agent_id(&self) -> String {
        self.agent_id.clone()
    }

    pub fn update_timestamp(&mut self, new_ts: i64) {
        self.last_ping_timestamp = new_ts
    }
    /// Shows the utilisation of tasks that an agent can handle.
    /// Agent tasks managers have internal queues for holding tasks
    /// if they have reached utilisation so we will allow negative values
    /// as the priority queue will naturally handle this
    pub fn utilisation(&self) -> usize {
        self.running_tasks
    }
    pub fn inc_running_tasks(&mut self) {
        self.running_tasks += 1
    }
    pub fn dec_running_tasks(&mut self) {
        if self.running_tasks > 0 {
            self.running_tasks -= 1
        }
    }
    pub fn get_last_ping_ts(&self) -> i64 {
        self.last_ping_timestamp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_meta_methods() {
        let mut agent = AgentMeta::new("localhost-9999".to_string(), 0);
        assert_eq!(agent.agent_id(), "localhost-9999");
        assert_eq!(agent.utilisation(), 0);

        agent.inc_running_tasks();
        assert_eq!(agent.utilisation(), 1);

        agent.inc_running_tasks();
        assert_eq!(agent.utilisation(), 2);

        agent.dec_running_tasks();
        assert_eq!(agent.utilisation(), 1);

        agent.dec_running_tasks();
        assert_eq!(agent.utilisation(), 0);

        agent.update_timestamp(10);
        assert_eq!(agent.get_last_ping_ts(), 10);
    }

    #[test]
    fn test_zmq_args() {
        let mut zmq_args = ZMQArgs::from(vec!["arg1".to_string(), "arg2".to_string()]);
        assert_eq!(zmq_args.len(), 2);

        assert_eq!(zmq_args.next(), Some("arg1".to_string()));
        assert_eq!(zmq_args.len(), 1);

        zmq_args.put("arg3".to_string());
        assert_eq!(zmq_args.len(), 2);

        let vec: Vec<String> = zmq_args.into();
        assert_eq!(vec, vec!["arg2".to_string(), "arg3".to_string()]);
    }

    #[test]
    fn test_zmqargs_from_string() {
        let zmq_args = ZMQArgs::from("arg1\x01arg2".to_string());
        assert_eq!(zmq_args.len(), 2);
    }

    #[test]
    fn test_zmqargs_from_string_with_backslashes() {
        let zmq_args = ZMQArgs::from("arg\\1\x01ar\\g2\x01\\\\".to_string());
        assert_eq!(zmq_args.len(), 3);
    }

    #[test]
    fn test_zmqargs_to_string() {
        let exp = "arg1\x01arg2\x01arg3 and space\x01arg4";
        let zmq_args = ZMQArgs::from(exp.to_string());
        assert_eq!(zmq_args.len(), 4);
        let st = zmq_args.to_string();
        assert_eq!(st, exp)
    }
}
