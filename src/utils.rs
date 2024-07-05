use std::{collections::VecDeque, sync::Arc};

use tokio::sync::Mutex;


pub fn arg_str_to_vec(s: &str) -> Vec<String> {
    s.split("|").map(|x|x.to_string()).collect()
}

/// A simple queue that can be accessed across threads. The queue should
/// holds an internal Arc<Mutex<T>> on the 
/// 
#[derive(Clone, Debug)]
pub struct AsyncQueue<T> {
    inner: Arc<Mutex<VecDeque<T>>>
}
impl <T>AsyncQueue<T> {
    pub fn new() -> Self {
        Self {inner: Arc::new(Mutex::new(VecDeque::new()))}
    }
    pub async fn get(&mut self) -> Option<T> {
        let mut queue = self.inner.lock().await;
        (*queue).pop_front()
    }
    pub async fn put(&mut self, item: T) {
        let mut queue = self.inner.lock().await;
        (*queue).push_back(item);
    } 
    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.is_empty()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arg_to_vec() {
        let args = "hello|world";
        assert_eq!(arg_str_to_vec(args), vec![
            "hello".to_string(), "world".to_string()
        ])
    }

    #[test]
    fn test_arg_to_vec_empty() {
        let args = "helloworld";
        assert_eq!(arg_str_to_vec(args), vec![
            "helloworld".to_string()
        ])
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