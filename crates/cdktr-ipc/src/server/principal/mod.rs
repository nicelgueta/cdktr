use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use cdktr_core::{
    models::AgentMeta,
    utils::data_structures::{AgentPriorityQueue, AsyncQueue},
};
use cdktr_db::DBClient;
use cdktr_workflow::{Workflow, WorkflowStore};
use chrono::Utc;

use cdktr_api::PrincipalAPI;
use log::{info, trace, warn};

use crate::log_manager::read_logs;

use super::traits::Server;
use cdktr_api::models::ClientResponseMessage;

pub mod helpers;

pub struct PrincipalServer {
    #[allow(dead_code)]
    instance_id: String,
    live_agents: AgentPriorityQueue,
    task_queue: AsyncQueue<Workflow>,
    workflows: WorkflowStore,
    db_client: DBClient,
    /// Maps agent_id to set of workflow_instance_ids currently running on that agent
    agent_workflows: Arc<tokio::sync::Mutex<HashMap<String, HashSet<String>>>>,
}

impl PrincipalServer {
    pub fn new(instance_id: String, workflows: WorkflowStore, db_client: DBClient) -> Self {
        Self {
            instance_id,
            live_agents: AgentPriorityQueue::new(),
            task_queue: AsyncQueue::new(),
            workflows,
            db_client,
            agent_workflows: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Registers the agent with the principal server. If it exists
    /// already then it simply updates with the latest timestamp
    async fn register_agent(&mut self, agent_id: &String) -> (ClientResponseMessage, usize) {
        let now = Utc::now().timestamp_micros();
        let update_result = self.live_agents.update_timestamp(agent_id, now).await;
        match update_result {
            Ok(_) => (),
            Err(_e) => {
                // agent not registered before so add new
                let agent_meta = AgentMeta::new(agent_id.clone(), now);
                self.live_agents.push(agent_meta).await
            }
        };
        (ClientResponseMessage::Success, 0)
    }

    /// Returns references to the agent tracking structures for heartbeat monitoring
    pub fn get_agent_tracking(
        &self,
    ) -> (
        AgentPriorityQueue,
        Arc<tokio::sync::Mutex<HashMap<String, HashSet<String>>>>,
        DBClient,
    ) {
        (
            self.live_agents.clone(),
            self.agent_workflows.clone(),
            self.db_client.clone(),
        )
    }
}

#[async_trait]
impl Server<PrincipalAPI> for PrincipalServer {
    async fn handle_client_message(
        &mut self,
        cli_msg: PrincipalAPI,
    ) -> (ClientResponseMessage, usize) {
        let result = match cli_msg {
            PrincipalAPI::Ping => (ClientResponseMessage::Pong, 0),
            PrincipalAPI::ListWorkflowStore => {
                helpers::handle_list_workflows(&self.workflows).await
            }
            PrincipalAPI::RunTask(task_id) => {
                helpers::handle_run_task(&task_id, &self.workflows, &mut self.task_queue).await
            }
            PrincipalAPI::RegisterAgent(agent_id) => self.register_agent(&agent_id).await,
            PrincipalAPI::WorkflowStatusUpdate(
                agent_id,
                workflow_id,
                workflow_instance_id,
                status,
            ) => {
                // Track agent-to-workflow mapping for heartbeat monitoring
                let mut agent_wf_map = self.agent_workflows.lock().await;
                match status {
                    cdktr_core::models::RunStatus::RUNNING => {
                        // Add workflow to agent's active set
                        agent_wf_map
                            .entry(agent_id.clone())
                            .or_insert_with(HashSet::new)
                            .insert(workflow_instance_id.clone());

                        // Increment running tasks counter for this agent
                        if let Err(e) = self.live_agents.update_running_tasks(&agent_id, true).await
                        {
                            warn!(
                                "Failed to increment running tasks for agent {}: {}",
                                agent_id,
                                e.to_string()
                            );
                        }
                    }
                    cdktr_core::models::RunStatus::COMPLETED
                    | cdktr_core::models::RunStatus::FAILED
                    | cdktr_core::models::RunStatus::CRASHED => {
                        // Remove workflow from agent's active set
                        if let Some(workflows) = agent_wf_map.get_mut(&agent_id) {
                            workflows.remove(&workflow_instance_id);
                            // Clean up empty entries
                            if workflows.is_empty() {
                                agent_wf_map.remove(&agent_id);
                            }
                        }

                        // Decrement running tasks counter for this agent
                        if let Err(e) = self
                            .live_agents
                            .update_running_tasks(&agent_id, false)
                            .await
                        {
                            warn!(
                                "Failed to decrement running tasks for agent {}: {}",
                                agent_id,
                                e.to_string()
                            );
                        }
                    }
                    _ => {} // PENDING, WAITING, etc - no tracking needed
                }
                drop(agent_wf_map);

                helpers::handle_agent_workflow_status_update(
                    self.db_client.clone(),
                    workflow_id,
                    workflow_instance_id,
                    status,
                )
                .await
            }
            PrincipalAPI::TaskStatusUpdate(
                _agent_id,
                task_id,
                task_instance_id,
                workflow_instance_id,
                status,
            ) => {
                // TODO do something with agent id
                helpers::handle_agent_task_status_update(
                    self.db_client.clone(),
                    task_id,
                    task_instance_id,
                    workflow_instance_id,
                    status,
                )
                .await
            }
            PrincipalAPI::FetchWorkflow(agent_id) => {
                helpers::handle_fetch_task(&mut self.task_queue, agent_id).await
            }
            PrincipalAPI::QueryLogs(end_ts, start_ts, wf_id, wf_ins_id, verbose) => {
                info!("Fetching logs");
                let logs_result =
                    read_logs(self.db_client.clone(), start_ts, end_ts, wf_id, wf_ins_id).await;
                match logs_result {
                    Ok(logs) => match serde_json::to_string(
                        &logs
                            .iter()
                            .map(|l| if verbose { l.format_full() } else { l.format() })
                            .collect::<Vec<String>>(),
                    ) {
                        Ok(str_result) => {
                            (ClientResponseMessage::SuccessWithPayload(str_result), 0)
                        }
                        Err(e) => (
                            ClientResponseMessage::ServerError(format!(
                                "Failed to read logs from db: {}",
                                e.to_string()
                            )),
                            0,
                        ),
                    },
                    Err(e) => (
                        ClientResponseMessage::ServerError(format!(
                            "Failed to read logs from db: {}",
                            e.to_string()
                        )),
                        0,
                    ),
                }
            }
            PrincipalAPI::GetRecentWorkflowStatuses => {
                helpers::handle_get_recent_workflow_statuses(self.db_client.clone()).await
            }
            PrincipalAPI::GetRegisteredAgents => {
                helpers::handle_get_registered_agents(self.live_agents.clone()).await
            }
        };
        trace!("Returning ({}): {}", result.1, result.0.to_string());
        result
    }
}

#[cfg(test)]
mod tests {
    use std::{thread::sleep, time::Duration};

    use cdktr_core::zmq_helpers::format_zmq_msg_str;
    use zeromq::ZmqMessage;

    use super::*;

    async fn get_workflowstore() -> WorkflowStore {
        WorkflowStore::from_dir("./test_artifacts/workflows")
            .await
            .unwrap()
    }

    #[test]
    fn test_principal_request_from_zmq_str_all_happy() {
        let regis_str = format_zmq_msg_str(vec!["REGISTERAGENT", "8999", "2"]);
        let all_happies = vec!["PING", "LSWORKFLOWS", &regis_str];
        for zmq_s in all_happies {
            let zmq_msg = ZmqMessage::from(zmq_s);
            let res = PrincipalAPI::try_from(zmq_msg);
            dbg!(&res);
            assert!(res.is_ok())
        }
    }

    #[tokio::test]
    async fn test_handle_cli_message_all_happy() {
        // e2e integration test of db crudvia the server
        let test_params: Vec<(&str, Box<dyn Fn(ClientResponseMessage) -> bool>, usize)> = vec![
            // ("PING", Box::new(|r: ClientResponseMessage| r == ClientResponseMessage::Pong), 0),
            (
                "LSWORKFLOWS",
                Box::new(|r: ClientResponseMessage| {
                    r == ClientResponseMessage::SuccessWithPayload("[]".to_string())
                }),
                0,
            ),
            (
                "REGISTERAGENT\x01localhost-8999\x013",
                Box::new(|r: ClientResponseMessage| r == ClientResponseMessage::Success),
                0,
            ),
        ];

        let mut server = PrincipalServer::new(
            "fake_ins".to_string(),
            get_workflowstore().await,
            DBClient::new(None).unwrap(),
        );
        for (zmq_s, assertion_fn, exp_exit_code) in test_params {
            println!("Testing {zmq_s}");
            let zmq_msg = ZmqMessage::from(zmq_s);
            let ar = PrincipalAPI::try_from(zmq_msg)
                .expect("Should be able to unwrap the agent from ZMQ command");
            let (resp, exit_code) = server.handle_client_message(ar).await;
            dbg!(&resp);
            assertion_fn(resp);
            assert_eq!(exit_code, exp_exit_code);
        }
    }

    #[tokio::test]
    async fn test_register_agent_new() {
        let mut server = PrincipalServer::new(
            "fake_ins".to_string(),
            get_workflowstore().await,
            DBClient::new(None).unwrap(),
        );
        let agent_id = String::from("localhost-4567");
        let (resp, exit_code) = server.register_agent(&agent_id).await;
        {
            server.live_agents.pop().await.unwrap();
        }
        assert!(resp == ClientResponseMessage::Success);
        assert!(exit_code == 0)
    }

    #[tokio::test]
    async fn test_register_agent_already_exists() {
        let mut server = PrincipalServer::new(
            "fake_ins".to_string(),
            get_workflowstore().await,
            DBClient::new(None).unwrap(),
        );
        let agent_id = String::from("localhost-4567");
        server.register_agent(&agent_id).await;
        let old_timestamp = { server.live_agents.pop().await.unwrap().get_last_ping_ts() };
        sleep(Duration::from_micros(10));
        let (resp, exit_code) = server.register_agent(&agent_id).await;
        let new_timestamp = { server.live_agents.pop().await.unwrap().get_last_ping_ts() };
        assert!(new_timestamp > old_timestamp);
        assert!(resp == ClientResponseMessage::Success);
        assert!(exit_code == 0)
    }

    #[tokio::test]
    async fn test_workflow_tracking_on_running_status() {
        let mut server = PrincipalServer::new(
            "fake_ins".to_string(),
            get_workflowstore().await,
            DBClient::new(None).unwrap(),
        );

        let agent_id = "test-agent-001".to_string();
        let workflow_id = "test-workflow".to_string();
        let workflow_instance_id = "test-instance-001".to_string();

        // Send RUNNING status update
        let msg = PrincipalAPI::WorkflowStatusUpdate(
            agent_id.clone(),
            workflow_id.clone(),
            workflow_instance_id.clone(),
            cdktr_core::models::RunStatus::RUNNING,
        );

        server.handle_client_message(msg).await;

        // Verify workflow is tracked
        let agent_wf_map = server.agent_workflows.lock().await;
        assert!(agent_wf_map.contains_key(&agent_id));
        assert!(
            agent_wf_map
                .get(&agent_id)
                .unwrap()
                .contains(&workflow_instance_id)
        );
    }

    #[tokio::test]
    async fn test_workflow_tracking_multiple_workflows() {
        let mut server = PrincipalServer::new(
            "fake_ins".to_string(),
            get_workflowstore().await,
            DBClient::new(None).unwrap(),
        );

        let agent_id = "test-agent-001".to_string();
        let workflow_id = "test-workflow".to_string();

        // Add multiple workflow instances
        for i in 1..=3 {
            let msg = PrincipalAPI::WorkflowStatusUpdate(
                agent_id.clone(),
                workflow_id.clone(),
                format!("test-instance-00{}", i),
                cdktr_core::models::RunStatus::RUNNING,
            );
            server.handle_client_message(msg).await;
        }

        // Verify all workflows are tracked
        let agent_wf_map = server.agent_workflows.lock().await;
        assert_eq!(agent_wf_map.get(&agent_id).unwrap().len(), 3);
    }

    #[tokio::test]
    async fn test_workflow_tracking_removal_on_completed() {
        let mut server = PrincipalServer::new(
            "fake_ins".to_string(),
            get_workflowstore().await,
            DBClient::new(None).unwrap(),
        );

        let agent_id = "test-agent-001".to_string();
        let workflow_id = "test-workflow".to_string();
        let workflow_instance_id = "test-instance-001".to_string();

        // Add workflow
        let msg_running = PrincipalAPI::WorkflowStatusUpdate(
            agent_id.clone(),
            workflow_id.clone(),
            workflow_instance_id.clone(),
            cdktr_core::models::RunStatus::RUNNING,
        );
        server.handle_client_message(msg_running).await;

        // Complete workflow
        let msg_completed = PrincipalAPI::WorkflowStatusUpdate(
            agent_id.clone(),
            workflow_id.clone(),
            workflow_instance_id.clone(),
            cdktr_core::models::RunStatus::COMPLETED,
        );
        server.handle_client_message(msg_completed).await;

        // Verify workflow is removed and agent entry is cleaned up
        let agent_wf_map = server.agent_workflows.lock().await;
        assert!(!agent_wf_map.contains_key(&agent_id));
    }

    #[tokio::test]
    async fn test_workflow_tracking_removal_on_failed() {
        let mut server = PrincipalServer::new(
            "fake_ins".to_string(),
            get_workflowstore().await,
            DBClient::new(None).unwrap(),
        );

        let agent_id = "test-agent-001".to_string();
        let workflow_id = "test-workflow".to_string();
        let workflow_instance_id = "test-instance-001".to_string();

        // Add workflow
        let msg_running = PrincipalAPI::WorkflowStatusUpdate(
            agent_id.clone(),
            workflow_id.clone(),
            workflow_instance_id.clone(),
            cdktr_core::models::RunStatus::RUNNING,
        );
        server.handle_client_message(msg_running).await;

        // Fail workflow
        let msg_failed = PrincipalAPI::WorkflowStatusUpdate(
            agent_id.clone(),
            workflow_id.clone(),
            workflow_instance_id.clone(),
            cdktr_core::models::RunStatus::FAILED,
        );
        server.handle_client_message(msg_failed).await;

        // Verify workflow is removed
        let agent_wf_map = server.agent_workflows.lock().await;
        assert!(!agent_wf_map.contains_key(&agent_id));
    }

    #[tokio::test]
    async fn test_workflow_tracking_partial_cleanup() {
        let mut server = PrincipalServer::new(
            "fake_ins".to_string(),
            get_workflowstore().await,
            DBClient::new(None).unwrap(),
        );

        let agent_id = "test-agent-001".to_string();
        let workflow_id = "test-workflow".to_string();

        // Add two workflows
        for i in 1..=2 {
            let msg = PrincipalAPI::WorkflowStatusUpdate(
                agent_id.clone(),
                workflow_id.clone(),
                format!("test-instance-00{}", i),
                cdktr_core::models::RunStatus::RUNNING,
            );
            server.handle_client_message(msg).await;
        }

        // Complete one workflow
        let msg_completed = PrincipalAPI::WorkflowStatusUpdate(
            agent_id.clone(),
            workflow_id.clone(),
            "test-instance-001".to_string(),
            cdktr_core::models::RunStatus::COMPLETED,
        );
        server.handle_client_message(msg_completed).await;

        // Verify agent still exists with one workflow
        let agent_wf_map = server.agent_workflows.lock().await;
        assert!(agent_wf_map.contains_key(&agent_id));
        assert_eq!(agent_wf_map.get(&agent_id).unwrap().len(), 1);
        assert!(
            agent_wf_map
                .get(&agent_id)
                .unwrap()
                .contains("test-instance-002")
        );
    }

    #[tokio::test]
    async fn test_get_agent_tracking_returns_correct_structures() {
        let server = PrincipalServer::new(
            "fake_ins".to_string(),
            get_workflowstore().await,
            DBClient::new(None).unwrap(),
        );

        let (_live_agents, agent_workflows, _db_client) = server.get_agent_tracking();

        // Verify we can access the returned structures
        tokio::spawn(async move {
            let map = agent_workflows.lock().await;
            assert_eq!(map.len(), 0); // Initially empty
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_get_registered_agents_empty() {
        let mut server = PrincipalServer::new(
            "fake_ins".to_string(),
            get_workflowstore().await,
            DBClient::new(None).unwrap(),
        );

        let (response, exit_code) = server
            .handle_client_message(PrincipalAPI::GetRegisteredAgents)
            .await;

        assert_eq!(exit_code, 0);
        match response {
            ClientResponseMessage::SuccessWithPayload(payload) => {
                let agents: Vec<cdktr_api::models::AgentInfo> =
                    serde_json::from_str(&payload).unwrap();
                assert_eq!(agents.len(), 0);
            }
            _ => panic!("Expected SuccessWithPayload"),
        }
    }

    #[tokio::test]
    async fn test_get_registered_agents_with_agents() {
        let mut server = PrincipalServer::new(
            "fake_ins".to_string(),
            get_workflowstore().await,
            DBClient::new(None).unwrap(),
        );

        // Register two agents
        let agent1_id = "agent-test-001".to_string();
        let agent2_id = "agent-test-002".to_string();

        server
            .handle_client_message(PrincipalAPI::RegisterAgent(agent1_id.clone()))
            .await;
        server
            .handle_client_message(PrincipalAPI::RegisterAgent(agent2_id.clone()))
            .await;

        // Get registered agents
        let (response, exit_code) = server
            .handle_client_message(PrincipalAPI::GetRegisteredAgents)
            .await;

        assert_eq!(exit_code, 0);
        match response {
            ClientResponseMessage::SuccessWithPayload(payload) => {
                let agents: Vec<cdktr_api::models::AgentInfo> =
                    serde_json::from_str(&payload).unwrap();
                assert_eq!(agents.len(), 2);

                let agent_ids: Vec<String> = agents.iter().map(|a| a.agent_id.clone()).collect();
                assert!(agent_ids.contains(&agent1_id));
                assert!(agent_ids.contains(&agent2_id));
            }
            _ => panic!("Expected SuccessWithPayload"),
        }
    }
}
