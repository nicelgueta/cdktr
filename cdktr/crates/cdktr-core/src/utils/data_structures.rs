use crate::{exceptions::GenericError, models::AgentMeta};
use std::{
    collections::{BinaryHeap, HashMap, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::{
    sync::Mutex,
    time::{sleep, Duration},
};

/// A simple queue that can be accessed across threads. The queue
/// holds an internal Arc<Mutex<T>> to abstract the verbose handling
/// of the mutex from the consumer
///
#[derive(Clone, Debug)]
pub struct AsyncQueue<T> {
    inner: Arc<Mutex<VecDeque<T>>>,
}
impl<T> AsyncQueue<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
    /// Gets the next item from the queue.
    pub async fn get(&mut self) -> Option<T> {
        let mut queue = self.inner.lock().await;
        (*queue).pop_front()
    }

    /// Puts an item on the queue
    pub async fn put(&mut self, item: T) {
        let mut queue = self.inner.lock().await;
        (*queue).push_back(item);
    }

    /// Checks whether the queue ois empty
    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.is_empty()
    }

    pub async fn size(&self) -> usize {
        self.inner.lock().await.len()
    }

    /// Similar to .get() but intead of returning an Option<T> it repeatedly polls
    /// the inner VecDeque until an item T is available
    /// TODO: loop currently set to 500 millis. this dependency needs to be inverted
    pub async fn get_wait(&mut self) -> T {
        loop {
            let item_res = {
                // scoped to release the lock before waiting
                let mut queue = self.inner.lock().await;
                (*queue).pop_front()
            };
            if let Some(t) = item_res {
                return t;
            } else {
                sleep(Duration::from_millis(500)).await;
                continue;
            }
        }
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
/// increase the space complexity of our data structure but benefit from O(log n) inserts instead
/// of O(n) recreations of the entire tree every time something is updated and amortised
/// O(1) pops. Since we prioritise speed over space for our principal, this is the chosen approach
#[derive(Clone, Debug)]
pub struct AgentPriorityQueue {
    // item(utilisation, unique_id)
    heap: Arc<Mutex<BinaryHeap<(usize, usize)>>>,

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
    /// with the utilisation generate from the agentmeta which serves as the key attribute
    /// determining its order in the queue. Then that unique id is inserted into a hasmap with
    /// the agent_id as the key so that the unique id can be easily retrieved when only the
    /// agent id is know like incoming requests from the agents to update their status
    pub async fn push(&mut self, agent_meta: AgentMeta) {
        let next_id = UNIQUENESS_COUNTER.fetch_add(1, Ordering::Relaxed);
        let agent_id = agent_meta.agent_id();
        // add utilisation and agent_id tuple to the max heap
        {
            let mut heap = self.heap.lock().await;
            (*heap).push((agent_meta.utilisation(), next_id));
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
            if let Some((_utilisation, unique_id)) = (*heap).pop() {
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
                return Err(GenericError::MissingAgents);
            }
        }
    }

    /// O(1) lookup to update the timestamp. This update doesn't affect its position in the queue
    /// so is done directly using the hashmap without having to update the uniqueness id
    pub async fn update_timestamp(
        &self,
        agent_id: &str,
        timestamp: i64,
    ) -> Result<(), GenericError> {
        let u_map = self.u_map.lock().await;
        match u_map.get(agent_id) {
            Some(unique_id) => {
                let mut node_map = self.node_map.lock().await;
                let agent_meta_res = node_map.get_mut(unique_id);
                match agent_meta_res {
                    Some(agent_meta) => {
                        agent_meta.update_timestamp(timestamp);
                        Ok(())
                    }
                    None => Err(GenericError::MissingAgents),
                }
            }
            None => return Err(GenericError::MissingAgents),
        }
    }
    /// removes an agentmeta from the queue in O(1) by removing it from the internal node_map which
    /// effectively marks it as stale on the heap. We also remove from the u_map because this could introduce a memory
    /// leak if the agent_ids changed regularly and thus the same ids were not re-used in this queue once the agentmeta
    /// is pushed back
    pub async fn remove(&mut self, agent_id: &str) -> Result<AgentMeta, GenericError> {
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
    /// O(log n) ID lookup to update agent utilisation which directly affects its position in the queue. This is
    /// time complexity can be acheived because we use the hashmap to mutably access the AgentMeta, make the update
    /// and then push back using .push() which is a O(log n) insert.
    pub async fn update_running_tasks(
        &mut self,
        agent_id: &str,
        up: bool,
    ) -> Result<(), GenericError> {
        let mut agent_meta = self.remove(agent_id).await?;
        match up {
            true => agent_meta.inc_running_tasks(),
            false => agent_meta.dec_running_tasks(),
        };
        self.push(agent_meta).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AgentMeta;
    use tokio::time::{sleep, timeout, Duration};

    #[tokio::test]
    async fn test_async_queue_new() {
        let queue: AsyncQueue<i32> = AsyncQueue::new();
        assert!(queue.is_empty().await);
    }

    #[tokio::test]
    async fn test_async_queue_put_and_get() {
        let mut queue: AsyncQueue<i32> = AsyncQueue::new();
        queue.put(1).await;
        assert!(!queue.is_empty().await);
        let item = queue.get().await;
        assert_eq!(item, Some(1));
        assert!(queue.is_empty().await);
    }

    #[tokio::test]
    async fn test_async_queue_put_and_get_wait() {
        let mut queue: AsyncQueue<i32> = AsyncQueue::new();
        // spawn another coroutine to wait for 500ms before putting item on
        // queue to check that the wait works
        let mut q_clone = queue.clone();
        tokio::spawn(async move {
            sleep(Duration::from_millis(200)).await;
            q_clone.put(1).await;
        });
        // check that nothing on queue - this check is almost certain to actually
        // occur before the 200 millis is up that the spawned task will take to
        // put something on the queue.
        // this is done after the spawn to prove that the item received on the queue
        // comes from another thread
        assert!(queue.is_empty().await);

        // wait on receipt of item
        let item = timeout(Duration::from_secs(1), queue.get_wait()).await;
        assert_eq!(item.unwrap(), 1);

        assert!(queue.is_empty().await);
    }

    #[tokio::test]
    async fn test_async_queue_is_empty() {
        let queue: AsyncQueue<i32> = AsyncQueue::new();
        assert!(queue.is_empty().await);
        let mut queue = queue;
        queue.put(42).await;
        assert!(!queue.is_empty().await);
        queue.get().await;
        assert!(queue.is_empty().await);
    }

    #[tokio::test]
    async fn test_async_queue_size() {
        let mut q = AsyncQueue::new();
        assert_eq!(q.size().await, 0);

        q.put(1).await;
        assert_eq!(q.size().await, 1);

        let _ = q.get_wait().await;
        assert_eq!(q.size().await, 0)
    }

    #[tokio::test]
    async fn test_is_empty() {
        let mut pq = AgentPriorityQueue::new();
        assert!(pq.is_empty().await);

        let agent = AgentMeta::new("localhost".to_string(), 9999, 0);
        pq.push(agent).await;
        assert!(!pq.is_empty().await);
    }

    #[tokio::test]
    async fn test_basic_apq() {
        let mut pq = AgentPriorityQueue::new();
        let mut agents = vec![
            AgentMeta::new("localhost".to_string(), 9997, 0),
            AgentMeta::new("localhost".to_string(), 9998, 0),
            AgentMeta::new("localhost".to_string(), 9999, 0),
        ];
        let simulated_utilisation: Vec<usize> = vec![2, 3, 1];
        for utilisation in simulated_utilisation {
            let mut ag_meta = agents.pop().unwrap();
            for _ in 0..(utilisation.clone() as i32) {
                ag_meta.inc_running_tasks();
            }
            pq.push(ag_meta.clone()).await
        }
        // utilisation for all should be max - 0 to start
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
        let mut agents = vec![
            AgentMeta::new("localhost".to_string(), 9996, 0),
            AgentMeta::new("localhost".to_string(), 9997, 0),
            AgentMeta::new("localhost".to_string(), 9998, 0),
            AgentMeta::new("localhost".to_string(), 9999, 0),
        ];
        let simulated_utilisation: Vec<usize> = vec![3, 4, 2, 1];
        for utilisation in simulated_utilisation {
            let mut ag_meta = agents.pop().unwrap();
            for _ in 0..(utilisation.clone() as i32) {
                ag_meta.inc_running_tasks();
            }
            pq.push(ag_meta.clone()).await
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
        let agents = vec![AgentMeta::new("somedude".to_string(), 9999, 0)];
        for ag_meta in agents {
            pq.push(ag_meta).await
        }
        let agent_id = "someotherdude-8999".to_string();
        assert!(pq.update_timestamp(&agent_id, 2).await.is_err());
    }

    #[tokio::test]
    async fn test_remove() {
        let mut pq = AgentPriorityQueue::new();
        let mut agents = vec![
            AgentMeta::new("not-edit".to_string(), 9996, 0),
            AgentMeta::new("not-edit".to_string(), 9997, 0),
            AgentMeta::new("to-edit".to_string(), 9998, 0),
            AgentMeta::new("not-edit".to_string(), 9999, 0),
        ];

        let simulated_utilisation: Vec<usize> = vec![3, 4, 2, 1];
        for utilisation in simulated_utilisation {
            let mut ag_meta = agents.pop().unwrap();
            for _ in 0..(utilisation.clone() as i32) {
                ag_meta.inc_running_tasks();
            }
            pq.push(ag_meta.clone()).await
        }

        let agent_id = "to-edit-9998";
        let agent_meta = pq
            .remove(agent_id)
            .await
            .expect("Should find the agent in the priority queue");
        assert_eq!(agent_meta.agent_id(), agent_id);
        while !pq.is_empty().await {
            let am = pq
                .pop()
                .await
                .expect("Should be able to pop if queue is not empty");
            // pop should handle the stale entry in the heap so
            // this item id should not ever match the one we removed
            // above
            assert_ne!(am.agent_id(), agent_id.to_string())
        }
    }
    #[tokio::test]
    async fn test_update_running_tasks_inc() {
        let mut pq = AgentPriorityQueue::new();
        let mut agents = vec![
            AgentMeta::new("localhost".to_string(), 9996, 0),
            AgentMeta::new("localhost".to_string(), 9997, 0),
            AgentMeta::new("localhost".to_string(), 9998, 0),
            AgentMeta::new("localhost".to_string(), 9999, 0),
        ];

        let simulated_utilisation: Vec<usize> = vec![1, 2, 3, 4];
        for utilisation in simulated_utilisation {
            let mut ag_meta = agents.pop().unwrap();
            for _ in 0..(utilisation.clone() as i32) {
                ag_meta.inc_running_tasks();
            }
            pq.push(ag_meta.clone()).await
        }

        // check increase task
        let _ = pq.update_running_tasks("localhost-9998", true).await;

        let am = pq.remove("localhost-9998").await.unwrap();
        assert_eq!(am.utilisation(), 3);
    }
    #[tokio::test]
    async fn test_update_running_tasks_dec() {
        let mut pq = AgentPriorityQueue::new();
        let mut already_running = AgentMeta::new("localhost".to_string(), 9996, 0);
        already_running.inc_running_tasks();

        assert_eq!(already_running.utilisation(), 1);
        let _ = pq.push(already_running).await;

        // put some other items in there
        let mut agents = vec![
            AgentMeta::new("localhost".to_string(), 9997, 0),
            AgentMeta::new("localhost".to_string(), 9998, 0),
            AgentMeta::new("localhost".to_string(), 9999, 0),
        ];

        let simulated_utilisation: Vec<usize> = vec![1, 2, 3];
        for utilisation in simulated_utilisation {
            let mut ag_meta = agents.pop().unwrap();
            for _ in 0..(utilisation.clone() as i32) {
                ag_meta.inc_running_tasks();
            }
            pq.push(ag_meta.clone()).await
        }

        // check decrease task
        pq.update_running_tasks("localhost-9996", false)
            .await
            .unwrap();

        let am = pq.remove("localhost-9996").await.unwrap();
        assert_eq!(am.utilisation(), 0); // back to 0 utilisation
    }

    #[tokio::test]
    async fn test_update_running_tasks_flow() {
        // this test should show that we can update the utilisation
        // of an item which then changes it's place in the queue
        let mut pq = AgentPriorityQueue::new();
        let mut agents = vec![
            AgentMeta::new("localhost".to_string(), 9996, 0),
            AgentMeta::new("localhost".to_string(), 9997, 0),
            AgentMeta::new("localhost".to_string(), 9998, 0),
            AgentMeta::new("localhost".to_string(), 9999, 0),
        ];

        let simulated_utilisation: Vec<usize> = vec![1, 2, 3, 4];
        for utilisation in simulated_utilisation {
            let mut ag_meta = agents.pop().unwrap();
            for _ in 0..(utilisation.clone() as i32) {
                ag_meta.inc_running_tasks();
            }
            pq.push(ag_meta.clone()).await
        }

        // assert 9996 with highest utilisation is top of the queue
        let am = pq.pop().await.expect("should be popping here");
        assert_eq!(am.agent_id(), "localhost-9996".to_string());
        pq.push(am).await;

        // simulate two tasks running on 9997 which now makes 9997 top of the queue
        pq.update_running_tasks("localhost-9997", true)
            .await
            .expect("Should be able to increase");

        pq.update_running_tasks("localhost-9997", true)
            .await
            .expect("Should be able to increase");

        // assert 9997 is now top of the queue
        let am = pq.pop().await.expect("should pop");
        assert_eq!(am.agent_id(), "localhost-9997".to_string());

        // assert 9997 does not appear in any further pops
        while !pq.is_empty().await {
            let am = pq
                .pop()
                .await
                .expect("Should be able to pop if queue is not empty");
            // pop should handle the stale entry in the heap so
            // this item id should not ever match the one we removed
            // above
            assert_ne!(am.agent_id(), "localhost-9997".to_string())
        }
    }
}
