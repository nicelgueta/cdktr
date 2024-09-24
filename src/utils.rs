use std::{collections::VecDeque, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::sleep};

pub fn arg_str_to_vec(s: String) -> VecDeque<String> {
    s.split("|").map(|x| x.to_string()).collect()
}

pub fn get_instance_id(host: &str, port: usize) -> String {
    let mut id = String::new();
    id.push_str(host);
    id.push_str("-");
    let port_s = port.to_string();
    id.push_str(&port_s);
    id
}

/// Splits an instance id into server and port
pub fn split_instance_id(id: &str) -> (String, usize) {
    let splits: Vec<&str> = id.split("-").collect();
    (
        splits[0].to_string(),
        splits[1].parse().expect(&format!(
            "Port does not appear to be a valid port number. Got: {}",
            splits[1]
        )),
    )
}

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

#[cfg(test)]
mod tests {
    use tokio::time::timeout;

    use super::*;

    #[test]
    fn test_arg_to_vec() {
        let args = "hello|world".to_string();
        assert_eq!(
            arg_str_to_vec(args),
            vec!["hello".to_string(), "world".to_string()]
        )
    }

    #[test]
    fn test_arg_to_vec_empty() {
        let args = "helloworld".to_string();
        assert_eq!(arg_str_to_vec(args), vec!["helloworld".to_string()])
    }

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
        // put something on the queue
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
}
