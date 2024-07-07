use std::time::Duration;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use zeromq::{Socket, SocketOptions, SocketRecv};
use async_trait::async_trait;
use crate::models::traits::EventListener;
use crate::{
    executors::get_executor,
    models::{
        Task,
        traits::Executor
    },
    utils::AsyncQueue
};

#[derive(Debug)]
pub struct TaskExecutionHandle {
    join_handle: JoinHandle<()>,
    stdout_receiver: mpsc::Receiver<String>
}
impl TaskExecutionHandle {
    pub fn new(join_handle: JoinHandle<()>, stdout_receiver: mpsc::Receiver<String>) -> Self {
        Self {
            join_handle, stdout_receiver
        }
    }
    pub async fn wait(self) -> Result<(), TaskManagerError> {
        match self.join_handle.await {
            Ok(_) => Ok(()),
            Err(e) => Err(TaskManagerError::FlowError)
        }
    }
    pub async fn wait_stdout(&mut self) -> Option<String> {
        self.stdout_receiver.recv().await
    }
    
}

#[derive(Debug, PartialEq)]
pub enum TaskManagerError {
    TooManyThreadsError,
    FlowError,
    // Other
}


/// `TaskManager` is a struct for managing and executing tasks concurrently within a specified limit of threads.
///
/// It is designed to queue tasks and manage their execution based on the availabilitsy of threads, ensuring that the number of concurrently running tasks does not exceed the specified maximum.
///
/// # Fields
/// - `instance_id`: A `String` identifier for the instance of `TaskManager`. This can be used to differentiate between multiple instances.
/// - `max_threads`: The maximum number of threads that can be used for executing tasks concurrently. This limit helps in controlling resource usage.
/// - `thread_counter`: An `Arc<Mutex<usize>>` that safely counts the number of active threads. This is shared across tasks to ensure thread-safe updates.
/// - `task_queue`: An `Arc<Mutex<VecDeque<Task>>>` that holds the tasks queued for execution. The use of `VecDeque` allows efficient task insertion and removal.
///
#[derive(Debug)]
pub struct TaskManager {
    instance_id: String,
    max_threads: usize,
    thread_counter: Arc<Mutex<usize>>,
    task_queue: AsyncQueue<Task>
}

impl TaskManager {
    pub fn new(instance_id: String, max_threads: usize, incoming_task_queue: AsyncQueue<Task>) -> Self {
        Self {
            instance_id,
            max_threads, 
            thread_counter: Arc::new(Mutex::new(0)),
            task_queue: incoming_task_queue
        }
    }

    /// This function takes a given task and runs it in the relevant executor depending on the type 
    /// of member of the Task enum it pertains to. 
    pub async fn run_in_executor(
        &mut self, 
        task: Task
    ) -> Result<TaskExecutionHandle,TaskManagerError> 
    {
        {
            if *self.thread_counter.lock().await >= self.max_threads {
                return Err(
                    TaskManagerError::TooManyThreadsError
                )
            };
        }
        let thread_counter: Arc<Mutex<usize>> = self.thread_counter.clone();
        let (tx, rx) = mpsc::channel(32);
        let executor  = get_executor(task);
        let handle = tokio::spawn(async move {
            // inform the TaskManager of another running process
            { // put in a scope to ensure the mutex lock is dropped
                let mut counter = thread_counter.lock().await;
                *counter+=1;
            }

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

        // pass the join handle and receiver up to the calling function for control of
        // the spwaned coroutine
        Ok(TaskExecutionHandle::new(handle, rx))
    }

    pub async fn start(&mut self, host: String, port: usize) {
        // create task queue
        let tqclone = self.task_queue.clone();

        let ins_id = self.instance_id.clone();
        let host_cl = host.clone();
        tokio::spawn(
            async move {
                let pub_listener = TaskManagerPubListener::new(
                    ins_id, host_cl, port
                );
                pub_listener.start_listening_loop(tqclone).await
            }
        );
        // spawn_task_execution_loop(task_queue)
        println!("TASKMANAGER-{}: Beginning task execution loop", self.instance_id);
        self.task_execution_loop().await
    }


    async fn task_execution_loop(&mut self) {
        loop {
            while self.task_queue.is_empty().await || *self.thread_counter.lock().await > self.max_threads {
                // if the queue is empty (no tasks to do) or the manager is currently running the
                // maxium allowes concurrent threads then just hang tight
                // println!("Waiting");
                sleep(Duration::from_micros(500)).await
            };
            let task = {
                self.task_queue.get().await.expect(&format!("TASKMANAGER-{}: Unable to pop task from queue", &self.instance_id))
            };
            let task_exe_result = self.run_in_executor(task).await;
            match task_exe_result {
                Err(e) => match e {
                    TaskManagerError::TooManyThreadsError => break,
                    _ => panic!("{}", &format!("TASKMANAGER-{}: Got TaskManagerError", self.instance_id))
                },
                Ok(mut task_exe) => {
                    // need to spawn the reading of the logs of the run task in order to free this thread
                    // to go back to looking at the queue
                    tokio::spawn(
                        async move {
                            while let Some(msg) = task_exe.wait_stdout().await {
                                println!("LOGGING: {}", msg);
                            };
                        }
                    );
                }
            }
        }
    }

}


async fn get_socket(host: &str, port: usize, instance_id: &str) -> zeromq::SubSocket {
    let options = SocketOptions::default();
    let mut socket = zeromq::SubSocket::with_options(options);
    socket
        .connect(&format!("tcp://{}:{}", host, port))
        .await
        .expect("TASKMANAGER-{instance_id}: Failed to connect");
    println!("TASKMANAGER-{instance_id}: connected to tcp://{}:{}", host, port);
    let subscription_string = format!("EXETASKDEF|{instance_id}");
    socket.subscribe(&subscription_string).await.expect("TASKMANAGER-{instance_id}: Failed to subscribe to subscription");

    socket
}

struct TaskManagerPubListener {
    instance_id: String,
    host: String,
    port: usize
}
impl TaskManagerPubListener {
    pub fn new(instance_id: String, host: String, port: usize) -> Self {
        Self { instance_id, host, port }
    }
}
#[async_trait]
impl EventListener<Task> for TaskManagerPubListener {
    async fn start_listening_loop(&self, mut out_queue: AsyncQueue<Task>) {
        println!("TASKMANAGER-{}: Subscribing to tcp://{}:{}", self.instance_id, self.host, self.port);
        let mut socket = get_socket(&self.host, self.port, &self.instance_id).await;
        println!("TASKMANAGER-{}: Successfully created SUB connection to tcp://{}:{}", self.instance_id, self.host, self.port);
        println!("TASKMANAGER-{}: Starting listening loop", self.instance_id);
        loop {
            let recv: zeromq::ZmqMessage = socket.recv().await.expect("Failed to get msg");
            let task_res = Task::try_from(recv);
            match task_res {
                Ok(task) => {
                    out_queue.put(task).await
    
                },
                Err(e) => println!("TASKMANAGER-{}: failed to parse ZMQ msg: {:?}", self.instance_id, e)
            }
        }
    }
}
// TODO: fix the broken pipe error
#[cfg(test)]
mod tests {
    use tokio::time::{sleep, Duration};
    use zeromq::ZmqMessage;
    use crate::taskmanager::TaskManagerError;
    use crate::models::Task;
    use crate::utils::AsyncQueue;

    use super::TaskManager;

    #[tokio::test]
    async fn test_run_single_flow() {
        let task = Task::try_from(ZmqMessage::from("TASKDEF|PROCESS|echo|test_run_flow")).expect(
            "Failed to create task from the ZMQMessage"
        );
        let mut zk = TaskManager::new("tm1".to_string(), 1, AsyncQueue::new());
        let result = zk.run_in_executor(task).await;
        assert!(result.is_ok());
        result.unwrap().wait().await.unwrap();
    }

    #[tokio::test]
    async fn test_run_single_flow_slow() {
        let mut zk = TaskManager::new("tm1".to_string(), 1, AsyncQueue::new());
        let task = Task::try_from(ZmqMessage::from("TASKDEF|PROCESS|python|s.py")).expect("Failed to create task from the ZMQMessage");
        let mut result = zk.run_in_executor(task).await;
        assert!(result.is_ok());
        let mut i = 0;
        while let Some(msg) = result.as_mut().unwrap().wait_stdout().await {
            let it_num = msg.parse::<i32>().unwrap();
            assert_eq!(it_num, i);
            i+=1;
        }   
        
    }

    #[tokio::test]
    async fn test_run_multiple_flow_slow() {
        let mut zk = TaskManager::new("tm1".to_string(), 3, AsyncQueue::new());
        let task1 = Task::try_from(ZmqMessage::from("TASKDEF|PROCESS|python|s.py|1")).expect("Failed to create task from the ZMQMessage");
        let task2= Task::try_from(ZmqMessage::from("TASKDEF|PROCESS|python|s.py|2")).expect("Failed to create task from the ZMQMessage");
        let task3 = Task::try_from(ZmqMessage::from("TASKDEF|PROCESS|python|s.py|1")).expect("Failed to create task from the ZMQMessage");
        let mut result1 = zk.run_in_executor(task1).await;
        let mut result2 = zk.run_in_executor(task2).await;
        let mut result3 = zk.run_in_executor(task3).await;
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert!(result3.is_ok());
        
        let mut i = 0;

        while let Some(msg) = result1.as_mut().unwrap().wait_stdout().await {
            let it_num = msg.parse::<i32>().unwrap();
            assert_eq!(it_num, i);
            i+=1;
        };
        i=0;
        while let Some(msg) = result2.as_mut().unwrap().wait_stdout().await {
            let it_num = msg.parse::<i32>().unwrap();
            assert_eq!(it_num, i);
            i+=1;
        };
        i=0;
        while let Some(msg) = result3.as_mut().unwrap().wait_stdout().await {
            let it_num = msg.parse::<i32>().unwrap();
            assert_eq!(it_num, i);
            i+=1;
        };

        
    }

    #[tokio::test]
    async fn test_run_multiple_flow_too_many_threads() {
        let mut zk = TaskManager::new("tm1".to_string(), 2, AsyncQueue::new());
        let task1 = Task::try_from(ZmqMessage::from("TASKDEF|PROCESS|python|s.py|1")).expect("Failed to create task from the ZMQMessage");
        let task2 = Task::try_from(ZmqMessage::from("TASKDEF|PROCESS|python|s.py|2")).expect("Failed to create task from the ZMQMessage");
        let result1 = zk.run_in_executor(task1).await;
        let result2 = zk.run_in_executor(task2).await;
        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let second = Duration::from_secs(1);
        sleep(second).await;
        let task3 = Task::try_from(ZmqMessage::from("TASKDEF|PROCESS|python|s.py|1")).expect("Failed to create task from the ZMQMessage");
        let result3 = zk.run_in_executor(task3).await;

        match result3 {
            Ok(_handle) => panic!("Adding another thread beyond max threads should error"),
            Err(e) => assert_eq!(e, TaskManagerError::TooManyThreadsError)
        }
    }
}