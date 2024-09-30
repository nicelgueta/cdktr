mod task;

use crate::{
    exceptions,
    server::models::RepReqError,
    utils::get_instance_id,
};
use std::collections::VecDeque;
pub use task::Task;
pub mod traits;

#[derive(Debug, PartialEq)]
pub enum FlowExecutionResult {
    SUCCESS,
    CRASHED(String),
    // ABORTED(String),
    // FAILURE(String),
}

impl FlowExecutionResult {
    pub fn _to_string(self) -> String {
        match self {
            Self::CRASHED(v) => v,
            _ => "".to_string(), // Self::ABORTED(v) => v,
                                 // Self::FAILURE(v) => v,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum TaskStatus {
    PENDING,
    RUNNING,
    WAITING,
    COMPLETED,
    FAILED,
}
impl TryFrom<String> for TaskStatus {
    type Error = RepReqError;
    fn try_from(value: String) -> Result<Self, RepReqError> {
        match value.as_str() {
            "PENDING" => Ok(TaskStatus::PENDING),
            "RUNNING" => Ok(TaskStatus::RUNNING),
            "WAITING" => Ok(TaskStatus::WAITING),
            "COMPLETED" => Ok(TaskStatus::COMPLETED),
            "FAILED" => Ok(TaskStatus::FAILED),
            _ => Err(RepReqError::ParseError(format!(
                "Unrecognised task status: {}",
                value
            ))),
        }
    }
}
impl TaskStatus {
    pub fn to_string(&self) -> String {
        match self {
            TaskStatus::PENDING => String::from("PENDING"),
            TaskStatus::RUNNING => String::from("RUNNING"),
            TaskStatus::WAITING => String::from("WAITING"),
            TaskStatus::COMPLETED => String::from("COMPLETED"),
            TaskStatus::FAILED => String::from("FAILED"),
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

/// Agent metadata held by principal that is used by the task router
/// to decide which agent to route tasks to and by the server to determine
/// status
#[derive(Clone, Debug)]
pub struct AgentMeta {
    pub host: String,
    pub port: usize,
    max_tasks: usize,
    running_tasks: usize,
    pub last_ping_timestamp: i64,
}
impl AgentMeta {
    pub fn new(host: String, port: usize, max_tasks: usize, last_ping_timestamp: i64) -> Self {
        Self {
            host,
            port,
            last_ping_timestamp,
            max_tasks,
            running_tasks: 0,
        }
    }
    pub fn agent_id(&self) -> String {
        get_instance_id(&self.host, self.port)
    }

    pub fn update_timestamp(&mut self, new_ts: i64) {
        self.last_ping_timestamp = new_ts
    }
    /// Shows the capacity of tasks that an agent can handle.
    /// Agent tasks managers have internal queues for holding tasks
    /// if they have reached capacity so we will allow negative values
    /// as the priority queue will naturally handle this
    pub fn capacity(&self) -> i32 {
        self.max_tasks as i32 - self.running_tasks as i32
    }
    pub fn inc_running_task(&mut self) {
        self.running_tasks += 1
    }
    pub fn dec_running_tasks(&mut self) {
        self.running_tasks -= 1
    }
    pub fn get_last_ping_ts(&self) -> i64 {
        self.last_ping_timestamp
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    
    

    #[tokio::test]
    async fn test_agent_meta_methods() {
        let mut agent = AgentMeta::new("localhost".to_string(), 9999, 2, 0);
        assert_eq!(agent.agent_id(), "localhost-9999");
        assert_eq!(agent.capacity(), 2);

        agent.inc_running_task();
        assert_eq!(agent.capacity(), 1);

        agent.dec_running_tasks();
        assert_eq!(agent.capacity(), 2);

        agent.update_timestamp(10);
        assert_eq!(agent.get_last_ping_ts(), 10);
    }

    #[tokio::test]
    async fn test_zmq_args() {
        let mut zmq_args = ZMQArgs::from(vec!["arg1".to_string(), "arg2".to_string()]);
        assert_eq!(zmq_args.len(), 2);

        assert_eq!(zmq_args.next(), Some("arg1".to_string()));
        assert_eq!(zmq_args.len(), 1);

        zmq_args.put("arg3".to_string());
        assert_eq!(zmq_args.len(), 2);

        let vec: Vec<String> = zmq_args.into();
        assert_eq!(vec, vec!["arg2".to_string(), "arg3".to_string()]);
    }
}
