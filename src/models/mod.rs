mod task;

use crate::{
    exceptions::{self, GenericError},
    server::models::RepReqError,
    utils::get_instance_id,
};
use std::{
    collections::{BinaryHeap, HashMap, VecDeque},
    sync::Arc,
};
pub use task::Task;
use tokio::sync::Mutex;
pub mod traits;
use std::sync::atomic::{AtomicUsize, Ordering};

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
    last_ping_timestamp: i64,
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

/// Priority queue used by the Task Router to keep track of agent task counts.
/// This needs to be slightly more complex than the standard max-heap priority queue
/// because the time when agents complete tasks cannot be known by the task router, so
/// we need a hashmap to be able to access agentmeta in O(1) time to make the update.
/// The issue with this is that we compromise the heap data structure by editing in-place
/// so to avoid this we'll simulate a decrease-key operation by marking the item as stale
/// and lazily removing it when/if it gets popped. We mark as stale by removing the item from the
/// hashmap, so staleness can be determined in O(1) at time of pop. Trade-off here is that we potentially
/// increases the space complexity of our data structure but benefit from O(log n) inserts instead
/// of O(n) recreations of the entire tree every time something is updated and amortised
/// O(1) pops. Since we prioritise speed over space for our principal, this is the chosen approach
#[derive(Clone, Debug)]
pub struct AgentPriorityQueue {
    // item(capacity, unique_id)
    heap: Arc<Mutex<BinaryHeap<(i32, usize)>>>,

    // unique_id to indicate staleness
    node_map: Arc<Mutex<HashMap<usize, AgentMeta>>>,

    // uniqueness_map to keep track of the latest
    // unique id thats being used for each agent
    u_map: Arc<Mutex<HashMap<String, usize>>>,
}

static UNIQUENESS_COUNTER: AtomicUsize = AtomicUsize::new(1);

impl AgentPriorityQueue {
    pub fn new() -> Self {
        Self {
            heap: Arc::new(Mutex::new(BinaryHeap::new())),
            node_map: Arc::new(Mutex::new(HashMap::new())),
            u_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub async fn is_empty(&self) -> bool {
        let heap = self.heap.lock().await;
        (*heap).is_empty()
    }

    /// push actually needs to handle 3 data structures.
    /// This needs to generate a unique id that is used
    /// as a key to the agentmeta hashmap. This key is pushed onto the max-heap along
    /// with the capacity generate from the agentmeta which serves as the key attribute
    /// determining its order in the queue. Then that unique id is inserted into a hasmap with
    /// the agent_id as the key so that the unique id can be easily retrieved when only the
    /// agent id is know like incoming requests from the agents to update their status
    pub async fn push(&mut self, agent_meta: AgentMeta) {
        let next_id = UNIQUENESS_COUNTER.fetch_add(1, Ordering::Relaxed);
        let agent_id = agent_meta.agent_id();
        // add capacity and agent_id tuple to the max heap
        {
            let mut heap = self.heap.lock().await;
            (*heap).push((agent_meta.capacity(), next_id));
        }

        // move agentmeta node to internal map
        {
            let mut node_map = self.node_map.lock().await;
            (*node_map).insert(next_id, agent_meta);
        }
        // move agent id to uniqueness map
        {
            let mut u_map = self.u_map.lock().await;
            (*u_map).insert(agent_id, next_id);
        }
    }

    /// O(1) pop of the item at the top of the queue and O(1) removals of the items respective
    /// items in both hashmaps
    pub async fn pop(&mut self) -> Result<AgentMeta, GenericError> {
        let mut heap = self.heap.lock().await;
        loop {
            if let Some((_capacity, unique_id)) = (*heap).pop() {
                // remove item from both the heap and agent_meta hashmap and return the agentmeta
                let agent_meta = {
                    let mut node_map = self.node_map.lock().await;

                    // check if item is stale, if so, skip and pop again
                    if !node_map.contains_key(&unique_id) {
                        continue;
                    };

                    node_map
                        .remove(&unique_id)
                        .expect("Failed to find node in map when it appeared in max heap")
                };

                let agent_id = agent_meta.agent_id();
                // remove from the uniqueness map
                {
                    let mut u_map = self.u_map.lock().await;
                    u_map.remove(&agent_id).expect(
                        "Failed to remove agent_id entry from u_map when is reference in max-heap",
                    );
                }
                return Ok(agent_meta);
            } else {
                return Err(exceptions::GenericError::MissingAgents);
            }
        }
    }

    /// O(1) lookup to update the timestamp. This update doesn't affect its position in the queue
    /// so is done directly using the hashmap without having to update the uniqueness id
    pub async fn update_timestamp(
        &self,
        agent_id: &String,
        timestamp: i64,
    ) -> Result<(), GenericError> {
        let unique_id = {
            let u_map = self.u_map.lock().await;
            match u_map.get(agent_id) {
                Some(unique_id) => unique_id.clone(),
                None => return Err(GenericError::MissingAgents)
            }
        };
        let mut node_map = self.node_map.lock().await;
        let agent_meta_res = node_map.get_mut(&unique_id);
        match agent_meta_res {
            Some(agent_meta) => {
                agent_meta.update_timestamp(timestamp);
                Ok(())
            }
            None => Err(GenericError::MissingAgents),
        }
    }
    /// removes an agentmeta from the queue in O(1) by removing it from the internal node_map which
    /// effectively marks it as stale on the heap. We also remove from the u_map because this could introduce a memory
    /// leak if the agent_ids changed regularly and thus the same ids were not re-used in this queue once the agentmeta
    /// is pushed back
    pub async fn remove(&mut self, agent_id: &String) -> Result<AgentMeta, GenericError> {
        let unique_id = {
            let mut u_map = self.u_map.lock().await;
            if let Some(id) = u_map.remove(agent_id) {
                id
            } else {
                return Err(GenericError::MissingAgents);
            }
        };
        let mut node_map = self.node_map.lock().await;
        let agent_meta_res = node_map.remove(&unique_id);
        match agent_meta_res {
            Some(agent_meta) => Ok(agent_meta),
            None => return Err(GenericError::MissingAgents),
        }
    }
    /// O(log n) ID lookup to update agent capacity which directly affects its position in the queue. This is
    /// time complexity can be acheived because we use the hashmap to mutably access the AgentMeta, make the update
    /// and then push back using .push() which is a O(log n) insert.
    pub async fn update_capacity(
        &mut self,
        agent_id: &String,
        decrease: bool,
    ) -> Result<(), GenericError> {
        let mut agent_meta = self.remove(agent_id).await?;
        match decrease {
            true => agent_meta.dec_running_tasks(),
            false => agent_meta.inc_running_task(),
        };
        self.push(agent_meta).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_is_empty() {
        let mut pq = AgentPriorityQueue::new();
        assert!(pq.is_empty().await);

        let agent = AgentMeta::new("localhost".to_string(), 9999, 2, 0);
        pq.push(agent).await;
        assert!(!pq.is_empty().await);
    }

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

    #[tokio::test]
    async fn test_basic_apq() {
        let mut pq = AgentPriorityQueue::new();
        let agents = vec![
            AgentMeta::new("localhost".to_string(), 9999, 2, 0),
            AgentMeta::new("localhost".to_string(), 9998, 3, 0),
            AgentMeta::new("localhost".to_string(), 9997, 1, 0),
        ];
        for ag_meta in agents {
            pq.push(ag_meta).await
        }
        // capacity for all should be max - 0 to start
        let top = pq.pop().await.unwrap();
        assert_eq!(&top.agent_id(), "localhost-9998");

        // not putting top back on the queue so next should be id1
        let top2 = pq.pop().await.unwrap();
        assert_eq!(&top2.agent_id(), "localhost-9999");
        // put back as is
        pq.push(top2).await;

        // check is back at top
        assert_eq!(&pq.pop().await.unwrap().agent_id(), "localhost-9999")
    }

    #[tokio::test]
    async fn test_update_timestamp() {
        let mut pq = AgentPriorityQueue::new();
        let agents = vec![
            AgentMeta::new("localhost".to_string(), 9999, 3, 0),
            AgentMeta::new("localhost".to_string(), 9998, 4, 0),
            AgentMeta::new("localhost".to_string(), 9997, 2, 0),
            AgentMeta::new("localhost".to_string(), 9996, 1, 0),
        ];
        for ag_meta in agents {
            pq.push(ag_meta).await
        }

        assert!(pq
            .update_timestamp(&"localhost-9997".to_string(), 2)
            .await
            .is_ok());

        // check is updated when accessed via pop
        // (third item)
        let _ = pq.pop().await;
        let _ = pq.pop().await;
        let id3 = pq.pop().await.unwrap();
        assert_eq!(id3.agent_id(), "localhost-9997".to_string());
        assert_eq!(id3.last_ping_timestamp, 2)
    }

    #[tokio::test]
    async fn test_update_timestamp_not_exist() {
        let mut pq = AgentPriorityQueue::new();
        let agents = vec![
            AgentMeta::new("somedude".to_string(), 9999, 3, 0),
        ];
        for ag_meta in agents {
            pq.push(ag_meta).await
        }
        let agent_id = "someotherdude-8999".to_string();
        assert!(pq.update_timestamp(&agent_id, 2).await.is_err());
    }

    #[tokio::test]
    async fn test_decrease_key(){
        let mut pq = AgentPriorityQueue::new();
        let agents = vec![
            AgentMeta::new("not-edit".to_string(), 9999, 3, 0),
            AgentMeta::new("to-edit".to_string(), 9998, 4, 0),
            AgentMeta::new("not-edit".to_string(), 9997, 2, 0),
            AgentMeta::new("not-edit".to_string(), 9996, 1, 0),
        ];
        for ag_meta in agents {
            pq.push(ag_meta).await
        }
    }
}
