use cdktr_api::{API, PrincipalAPI};
use cdktr_core::models::{FlowExecutionResult, RunStatus};
use cdktr_core::utils::get_principal_uri;
use cdktr_core::{exceptions::GenericError, models::traits::Executor};
use cdktr_workflow::Task;
use log::{debug, error, info, warn};
use rustyrs::EternalSlugGenerator;
use std::sync::Arc;
use std::time::Duration;
use task_tracker::TaskTracker;
use task_tracker::ThreadSafeTaskTracker;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::task::JoinSet;
use tokio::time::sleep;

use crate::log_manager::publisher::LogsPublisher;
use crate::server::principal;
use cdktr_api::PrincipalClient;
mod task_tracker;

const WAIT_TASK_SLEEP_INTERVAL_MS: Duration = Duration::from_millis(500);

#[derive(Debug)]
pub struct TaskExecutionHandle {
    #[allow(dead_code)]
    join_handle: JoinHandle<Result<(), TaskManagerError>>,
    stdout_receiver: mpsc::Receiver<String>,
    stderr_receiver: mpsc::Receiver<String>,
}
impl TaskExecutionHandle {
    pub fn new(
        join_handle: JoinHandle<Result<(), TaskManagerError>>,
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
    #[allow(dead_code)]
    TooManyThreadsError,
    FailedTaskError(String),
}
impl TaskManagerError {
    #[allow(dead_code)]
    pub fn to_string(&self) -> String {
        match self {
            Self::TooManyThreadsError => "Max threads reached".to_string(),
            Self::FailedTaskError(e) => e.clone(),
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
    pub async fn new(instance_id: String, max_concurrent_workflows: usize) -> Self {
        let principal_client = PrincipalClient::new(instance_id.clone())
            .await
            .expect("Failed to acquire principal client from within the Agent Task Manager");
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
                get_principal_uri()
            );
            return Err(e);
        }

        // Spawn heartbeat task to keep agent registered
        let heartbeat_client = self.principal_client.clone();
        let heartbeat_handle = tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(5)).await;
                if let Err(e) = heartbeat_client.send_heartbeat().await {
                    error!("Failed to send heartbeat to principal: {}", e.to_string());
                }
            }
        });

        info!(
            "TASKMANAGER-{}: Beginning task execution loop",
            self.instance_id
        );
        let loop_res = self.workflow_execution_loop().await;

        // Abort heartbeat task when workflow loop exits
        heartbeat_handle.abort();

        if let Err(e) = loop_res {
            //TODO: currently just aborts on errors - maybe split errors up into those that we should fully
            // abort on and others that are fine to re-engage the loop on?
            error!("{}", e.to_string());
            while *self.workflow_counter.lock().await > 0 {
                warn!(
                    "Tasks still running after principal loss - awaiting completion before aborting"
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
                .wait_next_workflow(WAIT_TASK_SLEEP_INTERVAL_MS)
                .await;
            let workflow: cdktr_workflow::Workflow = {
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
            // spawn workflow thread so we can return to request another workflow
            let agent_id = self.instance_id.clone();
            let workflow_id = workflow.id().clone();
            let principal_client = self.principal_client.clone();
            let _wf_handle: JoinHandle<Result<(), GenericError>> = tokio::spawn(async move {
                let workflow_instance_id = { name_gen_cl.lock().await.next() };
                if principal_client
                    .send(PrincipalAPI::WorkflowStatusUpdate(
                        agent_id.clone(),
                        workflow_id.clone(),
                        workflow_instance_id.clone(),
                        RunStatus::RUNNING,
                    ))
                    .await
                    .is_err()
                {
                    error!(
                        "Failed to send status update of RUNNING to principal for: {workflow_id}/{workflow_instance_id}"
                    )
                };
                let mut task_tracker = ThreadSafeTaskTracker::from_workflow(&workflow)?;
                if task_tracker.is_finished() {
                    warn!(
                        "Workflow {} doesn't have any tasks defined - skipping",
                        workflow.name()
                    );
                    return Ok(());
                }
                let mut read_handles = JoinSet::new();
                while !task_tracker.is_finished() {
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
                    principal_client
                        .send(PrincipalAPI::TaskStatusUpdate(
                            agent_id.clone(),
                            task_id.clone(),
                            task_execution_id.clone(),
                            workflow_instance_id.clone(),
                            RunStatus::PENDING,
                        ))
                        .await?;
                    let mut task_exe = loop {
                        let task_exe_result = run_in_executor(
                            principal_client.clone(),
                            task_tracker.clone(),
                            agent_id.clone(),
                            task_id.clone(),
                            task.clone(),
                            task_execution_id.clone(),
                            workflow_instance_id.clone(),
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
                                TaskManagerError::FailedTaskError(e) => {
                                    error!("{}", e);
                                    match task_tracker.mark_failed(&task_id) {
                                        Ok(_) => {
                                            error!(
                                                "Marked {}->{} as failure",
                                                task_id, task_execution_id
                                            );
                                        }
                                        Err(e) => {
                                            error!(
                                                "Error marking task as failure - aborting workflow"
                                            );
                                            if principal_client
                                                .send(PrincipalAPI::WorkflowStatusUpdate(
                                                    agent_id.clone(),
                                                    workflow_id.clone(),
                                                    workflow_instance_id.clone(),
                                                    RunStatus::CRASHED,
                                                ))
                                                .await
                                                .is_err()
                                            {
                                                error!(
                                                    "Failed to send status update of CRASHED to principal for: {workflow_id}/{workflow_instance_id}"
                                                )
                                            };
                                            return Err(e);
                                        }
                                    }
                                }
                            },
                        };
                    };
                    // need to spawn the reading of the logs of the run task in order to free this thread
                    // to go back to looking at the queue
                    let mut logs_pub = LogsPublisher::new(
                        workflow.id().clone(),
                        workflow.name().clone(),
                        workflow_instance_id.clone(),
                    )
                    .await?;
                    read_handles.spawn(async move {
                        let mut task_logger = logs_pub
                            .get_task_logger(&task_name, &task_execution_id)
                            .await;
                        while let Some(msg) = task_exe.wait_stdout().await {
                            let log_msg = format!("STDOUT {msg}");
                            info!("{}", &log_msg);
                            task_logger.info(&log_msg).await;
                        }
                        while let Some(msg) = task_exe.wait_stderr().await {
                            let log_msg = format!("STDERR {msg}");
                            error!("{}", &log_msg);
                            task_logger.error(&log_msg).await;
                        }
                        info!("Ended task {task_execution_id} ({task_name})");
                    });
                }
                read_handles.join_all().await;
                {
                    let mut counter = workflow_counter.lock().await;
                    debug!("Decrementing workflow counter (currently {})", *counter);
                    *counter -= 1;
                }
                info!(
                    "All tasks for workflow {}->{} complete",
                    workflow.name(),
                    workflow_instance_id,
                );
                match task_tracker.all_tasks_successful() {
                    true => {
                        info!(
                            "Workflow {}->{} completed successfully",
                            workflow.name(),
                            workflow_instance_id,
                        );
                        if principal_client
                            .send(PrincipalAPI::WorkflowStatusUpdate(
                                agent_id.clone(),
                                workflow_id.clone(),
                                workflow_instance_id.clone(),
                                RunStatus::COMPLETED,
                            ))
                            .await
                            .is_err()
                        {
                            error!(
                                "Failed to send status update of COMPLETED to principal for: {workflow_id}/{workflow_instance_id}"
                            )
                        };
                        Ok(())
                    }
                    false => {
                        warn!(
                            "Workflow {}->{} completed with failures",
                            workflow.name(),
                            workflow_instance_id,
                        );
                        if principal_client
                            .send(PrincipalAPI::WorkflowStatusUpdate(
                                agent_id.clone(),
                                workflow_id.clone(),
                                workflow_instance_id.clone(),
                                RunStatus::FAILED,
                            ))
                            .await
                            .is_err()
                        {
                            error!(
                                "Failed to send status update of FAILED to principal for: {workflow_id}/{workflow_instance_id}"
                            )
                        };
                        Ok(())
                    }
                }
            });
        }
    }
}

/// This function takes a given task and runs it in the relevant executor depending on the type
/// of member of the Task enum it pertains to.
async fn run_in_executor(
    principal_client: PrincipalClient,
    mut task_tracker: ThreadSafeTaskTracker,
    agent_id: String,
    task_id: String,
    task: Task,
    task_execution_id: String,
    workflow_instance_id: String,
) -> Result<TaskExecutionHandle, TaskManagerError> {
    let (handle, stdout_rx, stderr_rx) = {
        let (stdout_tx, stdout_rx) = mpsc::channel(32);
        let (stderr_tx, stderr_rx) = mpsc::channel(32);
        let executable_task = task.get_exe_task();
        let task_exe_id_clone = task_execution_id.clone();
        let workflow_ins_id_clone = workflow_instance_id.clone();
        let handle = tokio::spawn(async move {
            info!("Spawning task {task_exe_id_clone}");
            if principal_client
                .send(PrincipalAPI::TaskStatusUpdate(
                    agent_id.clone(),
                    task_id.clone(),
                    task_execution_id.clone(),
                    workflow_ins_id_clone.clone(),
                    RunStatus::RUNNING,
                ))
                .await
                .is_err()
            {
                error!(
                    "Failed to send status update of RUNNING to principal for task: {task_id}/{task_execution_id}"
                )
            };
            let flow_result = executable_task.run(stdout_tx, stderr_tx).await;
            match flow_result {
                FlowExecutionResult::SUCCESS => {
                    info!(
                        "Successfully completed task: {}->{}",
                        &task_id, &task_execution_id
                    );
                    if principal_client
                        .send(PrincipalAPI::TaskStatusUpdate(
                            agent_id.clone(),
                            task_id.clone(),
                            task_execution_id.clone(),
                            workflow_ins_id_clone.clone(),
                            RunStatus::COMPLETED,
                        ))
                        .await
                        .is_err()
                    {
                        error!(
                            "Failed to send status update of COMPLETED to principal for task: {task_id}/{task_execution_id}"
                        )
                    };
                    match task_tracker.mark_success(&task_id) {
                        Ok(_) => Ok(()),
                        Err(e) => Err(TaskManagerError::FailedTaskError(format!(
                            "Failed to mark task as success. Error: {}",
                            e.to_string()
                        ))),
                    }
                }
                FlowExecutionResult::FAILURE(err_msg) => {
                    error!(
                        "Task {}->{} experienced a critical failure. Error: {}",
                        &task_id, &task_execution_id, err_msg
                    );
                    if principal_client
                        .send(PrincipalAPI::TaskStatusUpdate(
                            agent_id.clone(),
                            task_id.clone(),
                            task_execution_id.clone(),
                            workflow_ins_id_clone.clone(),
                            RunStatus::FAILED,
                        ))
                        .await
                        .is_err()
                    {
                        error!(
                            "Failed to send status update of FAILED to principal for task: {task_id}/{task_execution_id}"
                        )
                    };
                    match task_tracker.mark_failed(&task_id) {
                        Ok(_) => {
                            warn!("Marked {}->{} as failure", &task_id, &task_execution_id);
                            Ok(())
                        }
                        Err(e) => Err(TaskManagerError::FailedTaskError(format!(
                            "Failed to mark task as success. Error: {}",
                            e.to_string()
                        ))),
                    }
                }
                FlowExecutionResult::CRASHED(err_msg) => {
                    error!(
                        "Task {}->{} crashed. Error: {}",
                        &task_id, &task_execution_id, err_msg
                    );
                    if principal_client
                        .send(PrincipalAPI::TaskStatusUpdate(
                            agent_id.clone(),
                            task_id.clone(),
                            task_execution_id.clone(),
                            workflow_ins_id_clone.clone(),
                            RunStatus::FAILED,
                        ))
                        .await
                        .is_err()
                    {
                        error!(
                            "Failed to send status update of CRASHED to principal for task: {task_id}/{task_execution_id}"
                        )
                    };
                    match task_tracker.mark_failed(&task_id) {
                        Ok(_) => {
                            warn!("Marked {}->{} as failure", &task_id, &task_execution_id);
                            Ok(())
                        }
                        Err(e) => Err(TaskManagerError::FailedTaskError(format!(
                            "Failed to mark task as success. Error: {}",
                            e.to_string()
                        ))),
                    }
                }
            }
        });
        (handle, stdout_rx, stderr_rx)
    };
    // pass the join handle and receiver up to the calling function for control of
    // the spawned coroutine
    Ok(TaskExecutionHandle::new(handle, stdout_rx, stderr_rx))
}

// TODO: fix the broken pipe error
#[cfg(test)]
mod tests {}
