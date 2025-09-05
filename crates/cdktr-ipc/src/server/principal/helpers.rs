use std::time::SystemTime;

use cdktr_api::models::{ClientResponseMessage, StausUpdate};
use cdktr_core::{
    models::{RunStatus, RunType},
    utils::data_structures::{AgentPriorityQueue, AsyncQueue},
};
use cdktr_db::DBClient;
use cdktr_workflow::{Workflow, WorkflowStore};
/// API module to provide all of the principal message handling
/// utilities
///
use log::{info, trace};

pub async fn handle_list_workflows(workflows: &WorkflowStore) -> (ClientResponseMessage, usize) {
    (
        ClientResponseMessage::SuccessWithPayload(workflows.to_string().await),
        0,
    )
}

pub async fn handle_agent_task_status_update(
    db_client: DBClient,
    task_id: String,
    task_instance_id: String,
    status: RunStatus,
) -> (ClientResponseMessage, usize) {
    let item = StausUpdate::new(
        task_id,
        task_instance_id,
        RunType::Task.to_string(),
        status.to_string(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    );
    let batch = vec![item];
    match db_client.batch_load("run_status", batch).await {
        Ok(()) => (ClientResponseMessage::Success, 0),
        Err(e) => (
            ClientResponseMessage::ServerError(format!("Failed to update task statuses: {:?}", e)),
            0,
        ),
    }
}

pub async fn handle_agent_workflow_status_update(
    db_client: DBClient,
    workflow_id: String,
    workflow_instance_id: String,
    status: RunStatus,
) -> (ClientResponseMessage, usize) {
    let item = StausUpdate::new(
        workflow_id,
        workflow_instance_id,
        RunType::Workflow.to_string(),
        status.to_string(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    );
    let batch = vec![item];
    match db_client.batch_load("run_status", batch).await {
        Ok(()) => (ClientResponseMessage::Success, 0),
        Err(e) => (
            ClientResponseMessage::ServerError(format!(
                "Failed to update workflow statuses: {:?}",
                e
            )),
            0,
        ),
    }
}

/// handler for the principal to place a workflow task on the queue ready for pick-up by a worker
pub async fn handle_run_task(
    workflow_id: &str,
    workflows: &WorkflowStore,
    queue: &mut AsyncQueue<Workflow>,
) -> (ClientResponseMessage, usize) {
    let task_id = workflow_id.to_string();
    let wf_res = workflows.get(&workflow_id).await;
    if let Some(wf) = wf_res {
        info!("Staging task -> {}", &workflow_id);
        queue.put(wf).await;
        info!("Current task queue size: {}", queue.size().await);
        (ClientResponseMessage::Success, 0)
    } else {
        info!("No workflow found with id {}. Cannot stage task", task_id);
        (
            ClientResponseMessage::ClientError(format!("No workflow exists with id {}", task_id)),
            0,
        )
    }
}

pub async fn handle_fetch_task(
    task_queue: &mut AsyncQueue<Workflow>,
    agent_id: String,
) -> (ClientResponseMessage, usize) {
    // TODO: do something with the agent ID like this agent is allowed to
    // process this type of task
    let task_res = task_queue.get().await;
    if let Some(task) = task_res {
        info!(
            "Agent {agent_id} requested workflow | Sending workflow -> {}",
            task.name(),
        );
        info!("Current task queue size: {}", task_queue.size().await);
        (
            ClientResponseMessage::SuccessWithPayload(task.to_string()),
            0,
        )
    } else {
        trace!("No task found - sending empty success to client");
        (ClientResponseMessage::Success, 0)
    }
}

#[cfg(test)]
mod tests {

    use cdktr_core::utils::data_structures::AsyncQueue;

    use super::*;

    #[test]
    fn test_handle_list_tasks_1_in_db() {
        // TODO
    }

    #[tokio::test]
    async fn test_handle_run_task() {
        // TODO
    }

    #[tokio::test]
    async fn test_fetch_task_no_tasks() {
        let mut task_queue: AsyncQueue<Workflow> = AsyncQueue::new();
        assert_eq!(task_queue.size().await, 0);

        let (cli_msg, code) = handle_fetch_task(&mut task_queue, "1234".to_string()).await;

        assert_eq!(task_queue.size().await, 0);
        assert_eq!(cli_msg, ClientResponseMessage::Success);
        assert_eq!(code, 0);
    }

    // TODO: more tests needed
}
