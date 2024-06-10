use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::sync::mpsc;

use crate::{
    executor::ProcessExecutor,
    interfaces::traits::Executor,
};

#[derive(Debug)]
pub struct TaskManager {
    max_threads: usize,
    thread_counter: Arc<Mutex<usize>>
}

#[derive(Debug, PartialEq)]
pub enum TaskManagerError {
    SpawnError(String),
    FlowError(String),
    Other
}
#[derive(Debug)]
struct Task {
    command: String,
    args: Option<Vec<String>>
}
impl Task {
    fn new(command: String, args: Option<Vec<String>>) -> Self {
        Self {
            command, args
        }
    }
}


// type TaskQueue: Arc<VecDeque<Task>>;

impl TaskManager {
    pub fn new(max_threads: usize) -> Self {
        Self {
            max_threads, 
            thread_counter: Arc::new(Mutex::new(0))
        }
    }
    /// Run a command in a spawned thread.
    /// Note that there is no join handle to wait on thread completion
    pub async fn run_in_executor(
        &mut self, 
        cmd: String, 
        args: Option<Vec<String>>,
    ) -> Result<mpsc::Receiver<String>,TaskManagerError> 
    {
        {
            if *self.thread_counter.lock().await >= self.max_threads {
                return Err(
                    TaskManagerError::SpawnError(
                        "Cannot spawn new process - max_threads reached".to_string()
                    )
                )
            };
        }
        let thread_counter: Arc<Mutex<usize>> = self.thread_counter.clone();
        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            // inform the TaskManager of another running process
            { // put in a scope to ensure the mutex lock is dropped
                let mut counter = thread_counter.lock().await;
                *counter+=1;
            }

            let executor = ProcessExecutor::new(&cmd, args);
            let _flow_result = executor.run(
                tx
            ).await;
            // TODO: handle the result

            
            // inform TaskManager process has terminated
            {
                let mut counter = thread_counter.lock().await;
                *counter-=1;
            }
        });
        Ok(rx)
    }

    pub fn start(&self) {
        // create task queue
        // spawn_zmq_loop(task_queue)
        // spawn_task_execution_loop(task_queue)
    }


}



// TODO: fix the broken pipe error
#[cfg(test)]
mod tests {
    use tokio::time::{sleep, Duration};

    use crate::taskmanager::TaskManagerError;

    use super::TaskManager;

    #[tokio::test]
    async fn test_run_single_flow() {
        let mut zk = TaskManager::new(1);
        let mut result = zk.run_in_executor("echo".to_string(), Some(vec!["Running test_run_flow".to_string()])).await;
        assert!(result.is_ok());
        while let Some(_) = result.as_mut().unwrap().recv().await {
            // don't do anything with the text in this one
            // just check it ran
        }   
    }

    #[tokio::test]
    async fn test_run_single_flow_slow() {
        let mut zk = TaskManager::new(1);
        let mut result = zk.run_in_executor("python".to_string(), Some(vec!["s.py".to_string(), "1".to_string()])).await;
        assert!(result.is_ok());
        let mut i = 0;
        while let Some(msg) = result.as_mut().unwrap().recv().await {
            let it_num = msg.parse::<i32>().unwrap();
            assert_eq!(it_num, i);
            i+=1;
        }   
        
    }

    #[tokio::test]
    async fn test_run_multiple_flow_slow() {
        let mut zk = TaskManager::new(3);
        let mut result1 = zk.run_in_executor("python".to_string(), Some(vec!["s.py".to_string(), "1".to_string()])).await;
        let mut result2 = zk.run_in_executor("python".to_string(), Some(vec!["s.py".to_string(), "2".to_string()])).await;
        let mut result3 = zk.run_in_executor("python".to_string(), Some(vec!["s.py".to_string(), "1".to_string()])).await;
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert!(result3.is_ok());
        
        let mut i = 0;

        while let Some(msg) = result1.as_mut().unwrap().recv().await {
            let it_num = msg.parse::<i32>().unwrap();
            assert_eq!(it_num, i);
            i+=1;
        };
        i=0;
        while let Some(msg) = result2.as_mut().unwrap().recv().await {
            let it_num = msg.parse::<i32>().unwrap();
            assert_eq!(it_num, i);
            i+=1;
        };
        i=0;
        while let Some(msg) = result3.as_mut().unwrap().recv().await {
            let it_num = msg.parse::<i32>().unwrap();
            assert_eq!(it_num, i);
            i+=1;
        };

        
    }

    #[tokio::test]
    async fn test_run_multiple_flow_too_many_threads() {
        let mut zk = TaskManager::new(2);
        let result1 = zk.run_in_executor("python".to_string(), Some(vec!["s.py".to_string(), "1".to_string()])).await;
        let result2 = zk.run_in_executor("python".to_string(), Some(vec!["s.py".to_string(), "2".to_string()])).await;
        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let second = Duration::from_secs(1);
        sleep(second).await;

        let result3 = zk.run_in_executor("python".to_string(), Some(vec!["s.py".to_string(), "1".to_string()])).await;

        match result3 {
            Ok(_handle) => panic!("Adding another thread beyond max threads should error"),
            Err(e) => assert_eq!(e, TaskManagerError::SpawnError("Cannot spawn new process - max_threads reached".to_string()))
        }
        
        
    }
}