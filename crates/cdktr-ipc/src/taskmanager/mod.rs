use crate::client::PrincipalClient;
use cdktr_core::{
    config::CDKTR_DEFAULT_TIMEOUT, exceptions::GenericError, models::traits::Executor,
};
use cdktr_workflow::{Task, Workflow};
use log::{debug, error, info, warn};
use rustyrs::EternalSlugGenerator;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use topological_sort::TopologicalSort;

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
        let executable_task = task.get_exe_task();
        let task_id_clone = task_execution_id.clone();
        let handle = tokio::spawn(async move {
            // inform the TaskManager of another running process
            {
                // put in a scope to ensure the mutex lock is dropped
                let mut counter = thread_counter.lock().await;
                *counter += 1;
            }
            info!("Spawning task {task_id_clone}");
            let _flow_result = executable_task.run(stdout_tx, stderr_tx).await;
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
        let loop_res = self.workflow_execution_loop().await;
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

    async fn workflow_execution_loop(&mut self) -> Result<(), GenericError> {
        loop {
            let workflow_result = self
                .principal_client
                .wait_next_workflow(WAIT_TASK_SLEEP_INTERVAL_MS, CDKTR_DEFAULT_TIMEOUT)
                .await;
            let workflow = match workflow_result {
                Ok(workflow) => workflow,
                Err(e) => {
                    error!("{}", e.to_string());
                    return Err(e);
                }
            };
            let mut task_tracker = self.create_task_tracker(&workflow);
            while task_tracker.len() > 0 {
                let next_tasks = task_tracker.pop_all();
                if next_tasks.is_empty() {
                    return Err(GenericError::RuntimeError(
                        "Invalid workflow - DAG contains a cycle".to_string(),
                    ));
                };
                let task_map = workflow.get_tasks();
                for task_id in next_tasks {
                    let task = task_map.get(&task_id).unwrap();
                    let task_exe_id = self.name_gen.next();
                    self.run_in_executor(task.clone(), task_exe_id).await; // TODO: do something with result
                }
            }
        }
    }

    fn create_task_tracker(&self, workflow: &Workflow) -> TopologicalSort<String> {
        let mut tp = TopologicalSort::new();
        for (task_id, task) in workflow.get_tasks() {
            match task.get_dependencies() {
                Some(deps) => {
                    for dep in deps {
                        tp.add_dependency(dep, task_id.clone());
                    }
                }
                None => (),
            }
        }
        tp
    }
}

// TODO: fix the broken pipe error
#[cfg(test)]
mod tests {
    use crate::taskmanager::TaskManagerError;
    use cdktr_core::models::ZMQArgs;
    use tokio::time::{sleep, Duration};

    use super::TaskManager;
}
