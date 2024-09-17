mod task;

use crate::{
    exceptions::{self, GenericError},
    utils::arg_str_to_vec,
};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, VecDeque},
    ops::DerefMut,
    sync::Arc,
};
pub use task::Task;
use tokio::sync::Mutex;
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

/// Agent metadata held by principal that is used by the task router
/// to decide which agent to route tasks to and by the server to determine
/// status
#[derive(Clone, Debug)]
pub struct AgentMeta {
    pub agent_id: String,
    max_tasks: usize,
    running_tasks: usize,
    last_ping_timestamp: i64,
}
impl AgentMeta {
    pub fn new(agent_id: String, max_tasks: usize, last_ping_timestamp: i64) -> Self {
        Self {
            agent_id,
            last_ping_timestamp,
            max_tasks,
            running_tasks: 0,
        }
    }

    pub fn update_timestamp(&mut self, new_ts: i64) {
        self.last_ping_timestamp = new_ts
    }
    /// Shows the capacity of tasks that an agent can handle.
    /// Agent tasks managers have internal queues for holding tasks
    /// if they have reached capacity so we will allow negative values
    /// as the priority queue will naturally handle this
    pub fn capacity(&self) -> i32 {
        (self.max_tasks - self.running_tasks) as i32
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

#[derive(Clone, Debug)]
pub struct AgentPriorityQueue {
    heap: Arc<Mutex<BinaryHeap<(i32, String)>>>,
    node_map: Arc<Mutex<HashMap<String, AgentMeta>>>,
}
impl AgentPriorityQueue {
    pub fn new() -> Self {
        Self {
            heap: Arc::new(Mutex::new(BinaryHeap::new())),
            node_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub async fn is_empty(&self) -> bool {
        let heap = self.heap.lock().await;
        (*heap).is_empty()
    }
    pub async fn push(&mut self, agent_meta: AgentMeta) {
        let agent_id = agent_meta.agent_id.clone();

        // add capcity and agent_id tuple to the max heap
        let mut heap = self.heap.lock().await;
        (*heap).push((agent_meta.capacity(), agent_id.clone()));

        // move agentmeta node to internal map
        let mut node_map = self.node_map.lock().await;
        (*node_map).insert(agent_id, agent_meta);
    }
    pub async fn pop(&mut self) -> Result<AgentMeta, exceptions::GenericError> {
        let mut heap = self.heap.lock().await;
        if let Some((_capacity, agent_id)) = (*heap).pop() {
            let mut node_map = self.node_map.lock().await;
            let agent_meta = node_map
                .remove(&agent_id)
                .expect("Failed to find node in map when it appeared in max heap");
            Ok(agent_meta)
        } else {
            Err(exceptions::GenericError::MissingAgents)
        }
    }
    pub async fn update_timestamp(
        &self,
        agent_id: &String,
        timestamp: i64,
    ) -> Result<(), GenericError> {
        let mut node_map = self.node_map.lock().await;
        let agent_meta_res = node_map.get_mut(agent_id);
        match agent_meta_res {
            Some(agent_meta) => {
                agent_meta.update_timestamp(timestamp);
                Ok(())
            }
            None => Err(GenericError::MissingAgents),
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

    #[tokio::test]
    async fn test_basic_apq() {
        let mut pq = AgentPriorityQueue::new();
        let agents = vec![
            AgentMeta::new("id1".to_string(), 2, 0),
            AgentMeta::new("id2".to_string(), 3, 0),
            AgentMeta::new("id3".to_string(), 1, 0),
        ];
        for ag_meta in agents {
            pq.push(ag_meta).await
        }
        // capacity for all should be max - 0 to start
        let top = pq.pop().await.unwrap();
        assert_eq!(&top.agent_id, "id2");

        // not putting top back on the queue so next should be id1
        let top2 = pq.pop().await.unwrap();
        assert_eq!(&top2.agent_id, "id1");
        // put back as is
        pq.push(top2).await;

        // check is back at top
        assert_eq!(&pq.pop().await.unwrap().agent_id, "id1")
    }

    #[tokio::test]
    async fn test_update_timestamp() {
        let mut pq = AgentPriorityQueue::new();
        let agents = vec![
            AgentMeta::new("id1".to_string(), 3, 0),
            AgentMeta::new("id2".to_string(), 4, 0),
            AgentMeta::new("id3".to_string(), 2, 0),
            AgentMeta::new("id4".to_string(), 1, 0),
        ];
        for ag_meta in agents {
            pq.push(ag_meta).await
        }

        assert!(pq.update_timestamp(&"id3".to_string(), 2).await.is_ok());

        // check is updated when accessed via pop
        // (third item)
        let _ = pq.pop().await;
        let _ = pq.pop().await;
        let id3 = pq.pop().await.unwrap();
        assert_eq!(id3.agent_id, "id3".to_string());
        assert_eq!(id3.last_ping_timestamp, 2)
    }
}
