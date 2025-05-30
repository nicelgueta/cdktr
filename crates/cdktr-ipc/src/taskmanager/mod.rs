use crate::client::PrincipalClient;
use cdktr_core::get_cdktr_setting;
use cdktr_core::{exceptions::GenericError, models::traits::Executor};
use cdktr_workflow::Task;
use log::{debug, error, info, warn};
use rustyrs::EternalSlugGenerator;
use std::time::Duration;
use std::{env, sync::Arc};
use task_tracker::TaskTracker;
use task_tracker::ThreadSafeTaskTracker;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::task::JoinSet;
use tokio::time::sleep;

mod task_tracker;

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
/// - `max_concurrency`: The maximum number of workflows that a single agent can handle simultaneously. Also applies to tasks within workflows
///     where multpple tasks can be executed in parallel.
/// - `workflow_counter`: An `Arc<Mutex<usize>>` that safely counts the number of active threads. This is shared across tasks to ensure thread-safe updates.
///
pub struct TaskManager {
    instance_id: String,
    max_concurrent_workflows: usize,
    workflow_counter: Arc<Mutex<usize>>,
    principal_client: PrincipalClient,
    name_gen: Arc<Mutex<EternalSlugGenerator>>,
}

impl TaskManager {
    pub async fn new(
        instance_id: String,
        max_concurrent_workflows: usize,
        principal_uri: String,
    ) -> Self {
        let principal_client = PrincipalClient::new(instance_id.clone(), principal_uri);
        Self {
            instance_id,
            max_concurrent_workflows,
            workflow_counter: Arc::new(Mutex::new(0)),
            principal_client,
            name_gen: Arc::new(Mutex::new(EternalSlugGenerator::new(2).unwrap())),
        }
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
            //TODO: currently just aborts on errors - maybe split errors up into those that we should fully
            // abort on and others that are fine to re-engage the loop on?
            error!("{}", e.to_string());
            while *self.workflow_counter.lock().await > 0 {
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
            {
                let counter = self.workflow_counter.lock().await;
                if *counter >= self.max_concurrent_workflows {
                    debug!("Max workflows reached - waiting for free slot before requesting");
                    sleep(WAIT_TASK_SLEEP_INTERVAL_MS).await;
                    continue;
                }
            }
            let workflow_counter = self.workflow_counter.clone();
            let workflow_result = self
                .principal_client
                .wait_next_workflow(
                    WAIT_TASK_SLEEP_INTERVAL_MS,
                    Duration::from_millis(
                        get_cdktr_setting!(CDKTR_DEFAULT_TIMEOUT_MS, usize) as u64
                    ),
                )
                .await;
            let workflow = {
                let mut counter = workflow_counter.lock().await;
                match workflow_result {
                    Ok(workflow) => {
                        debug!("Incrementing workflow counter (currently {})", *counter);
                        *counter += 1;
                        workflow
                    }
                    Err(e) => {
                        error!("{}", e.to_string());
                        return Err(e);
                    }
                }
            };
            debug!("MAX WF -> {}", self.max_concurrent_workflows);
            let name_gen_cl = self.name_gen.clone();
            let wf_handle: JoinHandle<Result<(), GenericError>> = tokio::spawn(async move {
                let workflow_id = { name_gen_cl.lock().await.next() };
                let mut task_tracker = ThreadSafeTaskTracker::from_workflow(&workflow)?;
                if task_tracker.is_empty() {
                    warn!(
                        "Workflow {} doesn't have any tasks defined - skipping",
                        workflow.name()
                    );
                    return Ok(());
                }
                let mut read_handles = JoinSet::new();
                while !task_tracker.is_empty() {
                    let task_id = if let Some(task_id) = task_tracker.get_next_task() {
                        task_id
                    } else {
                        debug!("All tasks busy - sleeping");
                        sleep(WAIT_TASK_SLEEP_INTERVAL_MS).await;
                        continue;
                    };
                    let task = (&workflow).get_task(&task_id).expect(
                        "Passed an incorrect task id to the workflow from the task mgr - this is a bug",
                    );
                    let task_execution_id = { name_gen_cl.lock().await.next() };
                    let task_name = task.name().to_string();

                    let mut task_exe = loop {
                        let task_exe_result = run_in_executor(
                            task_tracker.clone(),
                            task_id.clone(),
                            task.clone(),
                            task_execution_id.clone(),
                        )
                        .await;
                        match task_exe_result {
                            Ok(task_exe) => break task_exe,
                            Err(e) => match e {
                                TaskManagerError::TooManyThreadsError => {
                                    debug!("Max number of child threads reached - waiting..");
                                    sleep(Duration::from_millis(1000)).await;
                                    continue;
                                }
                            },
                        };
                    };
                    // need to spawn the reading of the logs of the run task in order to free this thread
                    // to go back to looking at the queue
                    read_handles.spawn(async move {
                        while let Some(msg) = task_exe.wait_stdout().await {
                            info!("{task_execution_id} | STDOUT | {msg}");
                        }
                        while let Some(msg) = task_exe.wait_stderr().await {
                            error!("{task_execution_id} | STDERR | {msg}");
                        }
                        info!("Completed task {task_execution_id} ({task_name})");
                    });
                }
                read_handles.join_all().await;
                {
                    let mut counter = workflow_counter.lock().await;
                    debug!("Decrementing workflow counter (currently {})", *counter);
                    *counter -= 1;
                }
                info!(
                    "All tasks for workflow {} ({}) complete",
                    workflow_id,
                    workflow.name()
                );
                Ok(())
            });
        }
    }
}

/// This function takes a given task and runs it in the relevant executor depending on the type
/// of member of the Task enum it pertains to.
async fn run_in_executor(
    mut task_tracker: ThreadSafeTaskTracker,
    task_id: String,
    task: Task,
    task_execution_id: String,
) -> Result<TaskExecutionHandle, TaskManagerError> {
    let (handle, stdout_rx, stderr_rx) = {
        let (stdout_tx, stdout_rx) = mpsc::channel(32);
        let (stderr_tx, stderr_rx) = mpsc::channel(32);
        let executable_task = task.get_exe_task();
        let task_exe_id_clone = task_execution_id.clone();
        let handle = tokio::spawn(async move {
            info!("Spawning task {task_exe_id_clone}");
            let _flow_result = executable_task.run(stdout_tx, stderr_tx).await;
            task_tracker.mark_complete(&task_id);
        });
        (handle, stdout_rx, stderr_rx)
    };
    // pass the join handle and receiver up to the calling function for control of
    // the spawned coroutine
    Ok(TaskExecutionHandle::new(handle, stdout_rx, stderr_rx))
}

// TODO: fix the broken pipe error
#[cfg(test)]
mod tests {
    use crate::taskmanager::TaskManagerError;
    use tokio::time::{sleep, Duration};

    use super::TaskManager;
}
