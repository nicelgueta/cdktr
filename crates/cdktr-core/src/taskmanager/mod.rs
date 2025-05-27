use crate::{
    client::PrincipalClient,
    config::CDKTR_DEFAULT_TIMEOUT,
    exceptions::GenericError,
    executors::get_executor,
    models::{traits::Executor, Task},
};
use log::{debug, error, info, warn};
use rustyrs::EternalSlugGenerator;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::sleep;

const WAIT_TASK_SLEEP_INTERVAL_MS: Duration = Duration::from_millis(500);

#[derive(Debug)]
pub struct TaskExecutionHandle {
    join_handle: JoinHandle<()>,
    stdout_receiver: mpsc::Receiver<String>,
    stderr_receiver: mpsc::Receiver<String>,
}
impl TaskExecutionHandle {
    pub fn new(
        join_handle: JoinHandle<()>,
        stdout_receiver: mpsc::Receiver<String>,
        stderr_receiver: mpsc::Receiver<String>,
    ) -> Self {
        Self {
            join_handle,
            stdout_receiver,
            stderr_receiver,
        }
    }

    pub async fn wait_stdout(&mut self) -> Option<String> {
        self.stdout_receiver.recv().await
    }
    pub async fn wait_stderr(&mut self) -> Option<String> {
        self.stderr_receiver.recv().await
    }
}

#[derive(Debug, PartialEq)]
pub enum TaskManagerError {
    TooManyThreadsError,
}
impl TaskManagerError {
    pub fn to_string(&self) -> String {
        match self {
            Self::TooManyThreadsError => "Max threads reached".to_string(),
        }
    }
}

/// `TaskManager` is a struct for managing and executing tasks concurrently within a specified limit of threads.
///
/// It is designed to queue tasks and manage their execution based on the availability of threads, ensuring that the number of concurrently running tasks does not exceed the specified maximum.
///
/// # Fields
/// - `instance_id`: A `String` identifier for the instance of `TaskManager`. This can be used to differentiate between multiple instances.
/// - `max_threads`: The maximum number of threads that can be used for executing tasks concurrently. This limit helps in controlling resource usage.
/// - `thread_counter`: An `Arc<Mutex<usize>>` that safely counts the number of active threads. This is shared across tasks to ensure thread-safe updates.
///
pub struct TaskManager {
    instance_id: String,
    max_threads: usize,
    thread_counter: Arc<Mutex<usize>>,
    principal_client: PrincipalClient,
    name_gen: EternalSlugGenerator,
}

impl TaskManager {
    pub async fn new(instance_id: String, max_threads: usize, principal_uri: String) -> Self {
        let principal_client = PrincipalClient::new(instance_id.clone(), principal_uri);
        Self {
            instance_id,
            max_threads,
            thread_counter: Arc::new(Mutex::new(0)),
            principal_client,
            name_gen: EternalSlugGenerator::new(2).unwrap(),
        }
    }

    /// This function takes a given task and runs it in the relevant executor depending on the type
    /// of member of the Task enum it pertains to.
    pub async fn run_in_executor(
        &mut self,
        task: Task,
        task_execution_id: String,
    ) -> Result<TaskExecutionHandle, TaskManagerError> {
        {
            if *self.thread_counter.lock().await >= self.max_threads {
                return Err(TaskManagerError::TooManyThreadsError);
            };
        }
        let thread_counter: Arc<Mutex<usize>> = self.thread_counter.clone();
        let (stdout_tx, stdout_rx) = mpsc::channel(32);
        let (stderr_tx, stderr_rx) = mpsc::channel(32);
        let executor = get_executor(task);
        let task_id_clone = task_execution_id.clone();
        let handle = tokio::spawn(async move {
            // inform the TaskManager of another running process
            {
                // put in a scope to ensure the mutex lock is dropped
                let mut counter = thread_counter.lock().await;
                *counter += 1;
            }
            info!("Spawning task {task_id_clone}");
            let _flow_result = executor.run(stdout_tx, stderr_tx).await;
            info!("Exiting task {task_id_clone}");
            // TODO: handle the result

            // inform TaskManager process has terminated
            {
                let mut counter = thread_counter.lock().await;
                *counter -= 1;
            }
        });

        // pass the join handle and receiver up to the calling function for control of
        // the spawned coroutine
        Ok(TaskExecutionHandle::new(handle, stdout_rx, stderr_rx))
    }

    pub async fn start(&mut self) -> Result<(), GenericError> {
        let register_result = self.principal_client.register_with_principal().await;
        if let Err(e) = register_result {
            error!(
                "Failed to register with principal host {}. Check host is available",
                self.principal_client.get_uri()
            );
            return Err(e);
        }

        info!(
            "TASKMANAGER-{}: Beginning task execution loop",
            self.instance_id
        );
        let loop_res = self.task_execution_loop().await;
        if let Err(e) = loop_res {
            error!("{}", e.to_string());
            while *self.thread_counter.lock().await > 0 {
                warn!(
                    "Tasks still running after principal loss- awaiting completion before aborting"
                );
                sleep(Duration::from_secs(10)).await;
            }
            Err(e)
        } else {
            Ok(())
        }
    }

    async fn task_execution_loop(&mut self) -> Result<(), GenericError> {
        loop {
            let task_result = self
                .principal_client
                .wait_next_task(WAIT_TASK_SLEEP_INTERVAL_MS, CDKTR_DEFAULT_TIMEOUT)
                .await;
            let task = match task_result {
                Ok(task) => task,
                Err(e) => {
                    error!("{}", e.to_string());
                    return Err(e);
                }
            };
            loop {
                let task_execution_id = self.name_gen.next();
                let task_exe_result: Result<TaskExecutionHandle, TaskManagerError> = self
                    .run_in_executor(task.to_owned(), task_execution_id.clone())
                    .await;
                match task_exe_result {
                    Err(e) => match e {
                        TaskManagerError::TooManyThreadsError => {
                            debug!("Max number of child threads reached - waiting..");
                            sleep(Duration::from_millis(1000)).await;
                            continue;
                        }
                    },
                    Ok(mut task_exe) => {
                        // need to spawn the reading of the logs of the run task in order to free this thread
                        // to go back to looking at the queue
                        tokio::spawn(async move {
                            while let Some(msg) = task_exe.wait_stdout().await {
                                info!("{task_execution_id} | STDOUT | {msg}");
                            }
                            while let Some(msg) = task_exe.wait_stderr().await {
                                error!("{task_execution_id} | STDERR | {msg}");
                            }
                        });
                        break;
                    }
                };
            }
        }
    }
}

// TODO: fix the broken pipe error
#[cfg(test)]
mod tests {
    use crate::models::{Task, ZMQArgs};
    use crate::taskmanager::TaskManagerError;
    use tokio::time::{sleep, Duration};

    use super::TaskManager;

    fn get_task(v: Vec<&str>) -> Task {
        let vec_s = v.iter().map(|x| x.to_string()).collect::<Vec<String>>();
        Task::try_from(ZMQArgs::from(vec_s)).expect("Failed to create task from the ZMQArgs")
    }

    #[tokio::test]
    async fn test_run_single_flow() {
        let task = get_task(vec!["PROCESS", "echo", "test_run_flow"]);
        let mut zk = TaskManager::new("tm1".to_string(), 1, "tcp://fake-uri".to_string()).await;
        let result = zk.run_in_executor(task, "fakeid".to_string()).await;
        assert!(result.is_ok());
        result.unwrap().wait_stdout().await.unwrap();
    }

    #[tokio::test]
    async fn test_run_single_flow_slow() {
        let mut zk = TaskManager::new("tm1".to_string(), 1, "tcp://fake-uri".to_string()).await;
        let task = get_task(vec!["PROCESS", "sleep", "1"]);
        let mut result = zk.run_in_executor(task, "fakeid".to_string()).await;
        assert!(result.is_ok());
        let mut i = 0;
        while let Some(msg) = result.as_mut().unwrap().wait_stdout().await {
            let it_num = msg.parse::<i32>().unwrap();
            assert_eq!(it_num, i);
            i += 1;
        }
    }

    #[tokio::test]
    async fn test_run_multiple_flow_slow() {
        let mut zk = TaskManager::new("tm1".to_string(), 1, "tcp://fake-uri".to_string()).await;
        let task1 = get_task(vec!["PROCESS", "sleep", "2"]);
        let task2 = get_task(vec!["PROCESS", "sleep", "2"]);
        let task3 = get_task(vec!["PROCESS", "sleep", "1"]);
        let mut result1 = zk.run_in_executor(task1, "fakeid".to_string()).await;
        let mut result2 = zk.run_in_executor(task2, "fakeid".to_string()).await;
        let mut result3 = zk.run_in_executor(task3, "fakeid".to_string()).await;
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert!(result3.is_ok());

        let mut i = 0;

        while let Some(msg) = result1.as_mut().unwrap().wait_stdout().await {
            let it_num = msg.parse::<i32>().unwrap();
            assert_eq!(it_num, i);
            i += 1;
        }
        i = 0;
        while let Some(msg) = result2.as_mut().unwrap().wait_stdout().await {
            let it_num = msg.parse::<i32>().unwrap();
            assert_eq!(it_num, i);
            i += 1;
        }
        i = 0;
        while let Some(msg) = result3.as_mut().unwrap().wait_stdout().await {
            let it_num = msg.parse::<i32>().unwrap();
            assert_eq!(it_num, i);
            i += 1;
        }
    }

    #[tokio::test]
    async fn test_run_multiple_flow_too_many_threads() {
        let mut zk = TaskManager::new("tm1".to_string(), 1, "tcp://fake-uri".to_string()).await;
        let task1 = get_task(vec!["PROCESS", "sleep", "2"]);
        let task2 = get_task(vec!["PROCESS", "sleep", "3"]);
        let result1 = zk.run_in_executor(task1, "fakeid".to_string()).await;
        let result2 = zk.run_in_executor(task2, "fakeid".to_string()).await;
        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let second = Duration::from_millis(1000);
        sleep(second).await;
        let task3 = get_task(vec!["PROCESS", "sleep", "1"]);
        let result3 = zk.run_in_executor(task3, "fakeid".to_string()).await;

        match result3 {
            Ok(_handle) => panic!("Adding another thread beyond max threads should error"),
            Err(e) => assert_eq!(e, TaskManagerError::TooManyThreadsError),
        }
    }
}
