use crate::server::models::ClientResponseMessage;
use cdktr_core::{
    models::TaskStatus,
    utils::data_structures::{AgentPriorityQueue, AsyncQueue},
};
use cdktr_workflow::{FromYaml, Workflow, Workflows};
/// API module to provide all of the principal message handling
/// utilities
///
use log::{info, trace};

pub fn handle_list_workflows(workflows: &Workflows<Workflow>) -> (ClientResponseMessage, usize) {
    (
        ClientResponseMessage::SuccessWithPayload(workflows.to_string()),
        0,
    )
}

pub async fn handle_agent_task_status_update(
    live_agents: AgentPriorityQueue,
    task_id: &str,
    status: &TaskStatus,
) -> (ClientResponseMessage, usize) {
    // TODO: do something with the task id.
    //
    // TODO
    (
        ClientResponseMessage::SuccessWithPayload("TBD".to_string()),
        0,
    )
}

/// handler for the principal to place a workflow task on the queue ready for pick-up by a worker
pub async fn handle_run_task<WF: FromYaml + Clone>(
    workflow_id: &str,
    workflows: &Workflows<WF>,
    queue: &mut AsyncQueue<WF>,
) -> (ClientResponseMessage, usize) {
    let task_id = workflow_id.to_string();
    info!("Staging task -> {}", &workflow_id);
    let wf_res = workflows.get(&workflow_id);
    if let Some(wf) = wf_res {
        queue.put((*wf).clone()).await;
        info!("Current task queue size: {}", queue.size().await);
        (ClientResponseMessage::Success, 0)
    } else {
        (
            ClientResponseMessage::ServerError(format!(
                "Failed to retreive task with id {}",
                task_id
            )),
            0,
        )
    }
}

pub async fn handle_fetch_task<WF: ToString>(
    task_queue: &mut AsyncQueue<WF>,
    agent_id: String,
) -> (ClientResponseMessage, usize) {
    // TODO: do something with the agent ID like this agent is allowed to
    // process this type of task
    let task_res = task_queue.get().await;
    if let Some(task) = task_res {
        info!(
            "Agent {agent_id} requested task | Sending task -> {}",
            task.to_string()
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
    use cdktr_workflow::testing::MockWorkflow;
    use serde_json::json;

    use super::*;

    #[test]
    fn test_handle_list_tasks_empty_db() {
        let wfs: Workflows<Workflow> = serde_json::from_value(json!({
            "dir": "/some/dir",
            "inner": {
                "wf1": {
                    "cron": "*/2 * * * * *",
                    "start_time":" 2025-01-20T12:30:00+00:00",
                    "tasks": {
                        "task1": {
                            "name": "Task 1",
                            "description": "Runs first task",
                            "config": {
                                "cmd": "echo",
                                "args": ["hello", "world"]
                            }
                        }
                    }

                }
            }
        }))
        .unwrap();
        let json_str = serde_json::to_string(&wfs).unwrap();
        assert_eq!(
            handle_list_workflows(&wfs),
            (ClientResponseMessage::SuccessWithPayload(json_str), 0)
        )
    }

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
        let mut task_queue: AsyncQueue<MockWorkflow> = AsyncQueue::new();
        assert_eq!(task_queue.size().await, 0);

        let (cli_msg, code) = handle_fetch_task(&mut task_queue, "1234".to_string()).await;

        assert_eq!(task_queue.size().await, 0);
        assert_eq!(cli_msg, ClientResponseMessage::Success);
        assert_eq!(code, 0);
    }

    #[tokio::test]
    async fn test_fetch_task_2_tasks() {
        let mut task_queue = AsyncQueue::new();

        // put some dummy tasks onthe queue
        task_queue
            .put(MockWorkflow {
                name: "fake1".to_string(),
            })
            .await;
        task_queue
            .put(MockWorkflow {
                name: "fake2".to_string(),
            })
            .await;

        assert_eq!(task_queue.size().await, 2);

        let (cli_msg, code) = handle_fetch_task(&mut task_queue, "1234".to_string()).await;

        assert_eq!(task_queue.size().await, 1);

        assert_eq!(
            cli_msg,
            ClientResponseMessage::SuccessWithPayload("PROCESS|echo|hello world".to_string())
        );
        assert_eq!(code, 0);

        assert_eq!(cli_msg.payload(), "PROCESS|echo|hello world".to_string())
    }
}
