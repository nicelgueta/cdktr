use std::time::SystemTime;

use cdktr_api::models::{ClientResponseMessage, StatusUpdate};
use cdktr_core::{
    exceptions::GenericError,
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
    let item = StatusUpdate::new(
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
    let item = StatusUpdate::new(
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

/// handler to get the latest status updates for the 10 most recent workflows
pub async fn handle_get_recent_workflow_statuses(
    db_client: DBClient,
) -> (ClientResponseMessage, usize) {
    // Query to get the latest status for each workflow, limited to 10 most recent
    // Using a window function to get the latest record per object_id
    let query = "
        WITH ranked_statuses AS (
            SELECT
                object_id,
                object_instance_id,
                run_type,
                status,
                timestamp_ms,
                ROW_NUMBER() OVER (PARTITION BY object_id ORDER BY timestamp_ms DESC) as rn
            FROM run_status
            WHERE run_type = 'Workflow'::RunType
        )
        SELECT
            object_id,
            object_instance_id,
            CAST(run_type AS VARCHAR) as run_type,
            CAST(status AS VARCHAR) as status,
            timestamp_ms
        FROM ranked_statuses
        WHERE rn = 1
        ORDER BY timestamp_ms DESC
        LIMIT 10
    ";

    let result = {
        let locked_client = db_client.lock_inner_client().await;
        let mut stmt = match locked_client.prepare(query) {
            Ok(s) => s,
            Err(e) => {
                return (
                    ClientResponseMessage::ServerError(format!("Failed to prepare query: {:?}", e)),
                    0,
                );
            }
        };

        stmt.query_map([], |row| {
            Ok(StatusUpdate::new(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .map_err(|e| GenericError::DBError(e.to_string()))
        .and_then(|rows| {
            rows.map(|r| r.map_err(|e| GenericError::DBError(e.to_string())))
                .collect::<Result<Vec<StatusUpdate>, GenericError>>()
        })
    };

    match result {
        Ok(status_updates) => match serde_json::to_string(&status_updates) {
            Ok(json) => (ClientResponseMessage::SuccessWithPayload(json), 0),
            Err(e) => (
                ClientResponseMessage::ServerError(format!(
                    "Failed to serialize status updates: {:?}",
                    e
                )),
                0,
            ),
        },
        Err(e) => (
            ClientResponseMessage::ServerError(format!("Database query failed: {:?}", e)),
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

    #[tokio::test]
    async fn test_get_recent_workflow_statuses() {
        use cdktr_core::models::RunStatus;

        let db_client = DBClient::new(None).unwrap();

        // Insert multiple workflow status updates
        let status_updates = vec![
            StatusUpdate::new(
                "workflow_1".to_string(),
                "instance_1a".to_string(),
                "Workflow".to_string(),
                RunStatus::RUNNING.to_string(),
                1234567890_u64,
            ),
            StatusUpdate::new(
                "workflow_1".to_string(),
                "instance_1b".to_string(),
                "Workflow".to_string(),
                RunStatus::COMPLETED.to_string(),
                1234567900_u64, // More recent
            ),
            StatusUpdate::new(
                "workflow_2".to_string(),
                "instance_2a".to_string(),
                "Workflow".to_string(),
                RunStatus::FAILED.to_string(),
                1234567895_u64,
            ),
        ];

        db_client
            .batch_load("run_status", status_updates.clone())
            .await
            .expect("Failed to insert status updates");

        // Test retrieving recent statuses
        let (response, code) = handle_get_recent_workflow_statuses(db_client.clone()).await;

        assert_eq!(code, 0);
        match response {
            ClientResponseMessage::SuccessWithPayload(payload) => {
                let retrieved: Vec<StatusUpdate> = serde_json::from_str(&payload).unwrap();

                // Should get 2 workflows (latest for each)
                assert_eq!(retrieved.len(), 2);

                // workflow_1 should have the most recent status (instance_1b)
                let wf1 = retrieved
                    .iter()
                    .find(|s| s.object_id() == "workflow_1")
                    .unwrap();
                assert_eq!(wf1.object_instance_id(), "instance_1b");
                assert_eq!(wf1.status(), &RunStatus::COMPLETED.to_string());

                // workflow_2 should have its only status
                let wf2 = retrieved
                    .iter()
                    .find(|s| s.object_id() == "workflow_2")
                    .unwrap();
                assert_eq!(wf2.object_instance_id(), "instance_2a");
                assert_eq!(wf2.status(), &RunStatus::FAILED.to_string());
            }
            _ => panic!("Expected SuccessWithPayload, got {:?}", response),
        }
    }

    #[tokio::test]
    async fn test_get_recent_workflow_statuses_no_results() {
        let db_client = DBClient::new(None).unwrap();

        let (response, code) = handle_get_recent_workflow_statuses(db_client).await;

        assert_eq!(code, 0);
        match response {
            ClientResponseMessage::SuccessWithPayload(payload) => {
                let retrieved: Vec<StatusUpdate> = serde_json::from_str(&payload).unwrap();
                assert_eq!(retrieved.len(), 0);
            }
            _ => panic!("Expected SuccessWithPayload, got {:?}", response),
        }
    }

    // TODO: more tests needed
}
