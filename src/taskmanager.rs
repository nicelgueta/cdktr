use std::time::Duration;
use std::{collections::VecDeque, sync::Arc};
use diesel::IntoSql;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use zeromq::{Socket, SocketOptions, SocketRecv};

use crate::{
    executor::ProcessExecutor,
    interfaces::{
        Task,
        traits::Executor
    },
};

#[derive(Debug)]
pub struct TaskManager {
    instance_id: String,
    max_threads: usize,
    thread_counter: Arc<Mutex<usize>>,
    task_queue: Arc<Mutex<VecDeque<Task>>>
}

#[derive(Debug, PartialEq)]
pub enum TaskManagerError {
    TooManyThreadsError,
    FlowError,
    Other
}

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

// type TaskQueue: Arc<VecDeque<Task>>;

impl TaskManager {
    pub fn new(instance_id: String, max_threads: usize) -> Self {
        Self {
            instance_id,
            max_threads, 
            thread_counter: Arc::new(Mutex::new(0)),
            task_queue: Arc::new(Mutex::new(VecDeque::new()))
        }
    }

    pub async fn run_in_executor(
        &mut self, 
        cmd: String, 
        args: Option<Vec<String>>,
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

        let handle = tokio::spawn(async move {
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
        Ok(TaskExecutionHandle::new(handle, rx))
    }

    pub async fn start(&mut self, host: String, port: usize) {
        // create task queue
        let tqclone = self.task_queue.clone();

        let ins_id = self.instance_id.clone();
        tokio::spawn(
            async move {
                zmq_loop(&ins_id, host, port, tqclone).await
            }
        );
        // spawn_task_execution_loop(task_queue)
        println!("TASKMANAGER: Beginning task execution loop");
        self.task_execution_loop().await
    }
    async fn task_execution_loop(&mut self) {
        loop {
            while self.task_queue.lock().await.is_empty() || *self.thread_counter.lock().await > self.max_threads {
                // if the queue is empty (no tasks to do) or the manager is currently running the
                // maxium allowes concurrent threads then just hang tight
                // println!("Waiting");
                sleep(Duration::from_micros(500)).await
            };
            let task = {
                self.task_queue.lock().await.pop_front().expect("TASKMANAGER: Unable to pop task from queue")
            };
            let task_exe_result = self.run_in_executor(task.command, task.args).await;
            match task_exe_result {
                Err(e) => match e {
                    TaskManagerError::TooManyThreadsError => break,
                    _ => panic!("TASKMANAGER: Got TaskManagerError")
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
        .expect("TASKMANAGER: Failed to connect");
    println!("TASKMANAGER: connected to tcp://{}:{}", host, port);
    socket.subscribe(instance_id).await.expect("TASKMANAGER: Failed to subscribe to subscription");
    socket
}
/// This function is used to listen to the ZMQ socket and push the messages to the task queue
/// Currently this takes any command and args and pushes them to the task queue
/// TODO: this should instead receive an ID of a flow that has been registered
/// and then query the database for the flow and push that to the task queue. 
/// 
pub async fn zmq_loop(instance_id: &str, host: String, port: usize, task_queue_mutex: Arc<Mutex<VecDeque<Task>>>){

    println!("TASKMANAGER: Subscribing to tcp://{}:{}", host, port);
    let mut socket = get_socket(&host, port, instance_id).await;
    println!("TASKMANAGER: Successfully created SUB connection to tcp://{}:{}", host, port);
    println!("TASKMANAGER: Starting listening loop");
    loop {
        let recv: zeromq::ZmqMessage = socket.recv().await.expect("Failed to get msg");
        let msg = String::try_from(recv).unwrap();
        let cmd: Vec<String> = msg.split("|").into_iter().map(|x| x.to_string()).collect();
        let command = cmd[0].clone();
        let args = if cmd.len() > 1 {
            Some(cmd[1..].iter().map(|x| x.clone()).collect())
        } else {
            None
        };
        let task = Task {instance_id: instance_id.to_string(), command, args};
        {
            let mut task_queue = task_queue_mutex.lock().await;
            (*task_queue).push_back(task);
        }
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
        let mut zk = TaskManager::new("tm1".to_string(), 1);
        let result = zk.run_in_executor("echo".to_string(), Some(vec!["Running test_run_flow".to_string()])).await;
        assert!(result.is_ok());
        result.unwrap().wait().await.unwrap();
    }

    #[tokio::test]
    async fn test_run_single_flow_slow() {
        let mut zk = TaskManager::new("tm1".to_string(), 1);
        let mut result = zk.run_in_executor("python".to_string(), Some(vec!["s.py".to_string(), "1".to_string()])).await;
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
        let mut zk = TaskManager::new("tm1".to_string(), 3);
        let mut result1 = zk.run_in_executor("python".to_string(), Some(vec!["s.py".to_string(), "1".to_string()])).await;
        let mut result2 = zk.run_in_executor("python".to_string(), Some(vec!["s.py".to_string(), "2".to_string()])).await;
        let mut result3 = zk.run_in_executor("python".to_string(), Some(vec!["s.py".to_string(), "1".to_string()])).await;
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
        let mut zk = TaskManager::new("tm1".to_string(), 2);
        let result1 = zk.run_in_executor("python".to_string(), Some(vec!["s.py".to_string(), "1".to_string()])).await;
        let result2 = zk.run_in_executor("python".to_string(), Some(vec!["s.py".to_string(), "2".to_string()])).await;
        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let second = Duration::from_secs(1);
        sleep(second).await;

        let result3 = zk.run_in_executor("python".to_string(), Some(vec!["s.py".to_string(), "1".to_string()])).await;

        match result3 {
            Ok(_handle) => panic!("Adding another thread beyond max threads should error"),
            Err(e) => assert_eq!(e, TaskManagerError::TooManyThreadsError)
        }
    }
}