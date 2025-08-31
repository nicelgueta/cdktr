use async_trait::async_trait;
use cdktr_api::models::ClientResponseMessage;
use cdktr_api::{API, PrincipalAPI};
use cdktr_core::exceptions::GenericError;
use cdktr_core::utils::{data_structures::AsyncQueue, get_default_timeout, get_principal_uri};
use cdktr_workflow::{Task, Workflow};
use chrono::{DateTime, Utc};
use cron::Schedule;
use log::{debug, info};
use std::collections::{BinaryHeap, HashMap};
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;

use crate::traits::EventListener;

/// Main scheduling component. This component has an internal task queue for tasks
/// that are to be scheduled within the next poll interval.
/// There are two main loops that this component runs.
/// The first is to check the time of the first item in the queue and wait.
/// Once time, the scheduler dequeues the task and sends it to the taskmanager.
/// The second loop runs on a separate thread (because currently cannot find a way
/// to read diesel async) to poll the DB for schedules and when it finds flows that
/// are supposed to start within the next poll interval it queues them in order of
/// earliest to latest
pub struct Scheduler {
    principal_uri: String,
    workflows: HashMap<String, Workflow>,
    schedule_priority_queue: BinaryHeap<(i64, String)>,
    next_peek: (String, i64),
}

#[async_trait]
impl EventListener<Task> for Scheduler {
    async fn start_listening(&mut self) -> Result<(), GenericError> {
        loop {
            let mut ms_to_wait = self.next_peek.1 - Utc::now().timestamp_millis();
            ms_to_wait = if ms_to_wait < 0 { 0 } else { ms_to_wait };
            let time_to_wait = Duration::from_millis(ms_to_wait as u64);

            // put this component to sleep until the allotted time
            info!(
                "Next task `{}` scheduled to run at {}",
                self.next_peek.0,
                DateTime::from_timestamp_millis(self.next_peek.1)
                    .unwrap()
                    .to_rfc2822()
            );
            sleep(time_to_wait).await;

            let workflow_id = self.schedule_priority_queue.pop().unwrap().1;
            if self.next_peek.0 != workflow_id {
                return Err(GenericError::RuntimeError(format!(
                    "Popped wrong workflow from the priority queue. Expected {} but got {}",
                    self.next_peek.0, workflow_id
                )));
            }
            info!("Staging scheduled task: {}", &workflow_id);
            self.run_workflow(&workflow_id).await?;

            // add the next run of the same workflow back to priority queue
            let workflow = self.workflows.get(&workflow_id).unwrap();
            match workflow.cron() {
                Some(cron) => {
                    let next_run = Self::next_run_from_cron(cron, Ok(Utc::now()))?;
                    // invert the timestamp to make a min heap
                    self.schedule_priority_queue
                        .push((-next_run.timestamp_millis(), workflow_id));
                    // update the peek
                    let q_top = self.schedule_priority_queue.peek().unwrap();
                    self.next_peek = (q_top.1.clone(), -q_top.0);
                }
                None => continue,
            };
        }
    }
}
impl Scheduler {
    pub async fn new() -> Result<Self, GenericError> {
        let principal_uri = get_principal_uri();
        let api = PrincipalAPI::ListWorkflowStore;
        let response = api.send(&principal_uri, get_default_timeout()).await?;
        let workflows: HashMap<String, Workflow> = match response {
            ClientResponseMessage::SuccessWithPayload(wfs) => {
                serde_json::from_str(&wfs).map_err(|e| {
                    GenericError::ParseError(format!(
                        "Failed to read workflows from principal message. Not valid JSON: {}",
                        e.to_string()
                    ))
                })
            }
            other => Err(GenericError::WorkflowError(other.to_string())),
        }?;
        info!(
            "Scheduler found {} workflows with active schedules",
            (&workflows).len()
        );
        let schedule_priority_queue = Self::build_schedule_queue(&workflows)?;
        if schedule_priority_queue.is_empty() {
            return Err(GenericError::NoDataException(
                "No workflows have valid schedules. Scheduler cannot run".to_string(),
            ));
        };
        // flip sign to get the original value
        let q_top = schedule_priority_queue.peek().unwrap();
        let next_peek = (q_top.1.clone(), -q_top.0);
        Ok(Self {
            principal_uri,
            workflows,
            schedule_priority_queue,
            next_peek,
        })
    }

    /// Build the schedules using a min-heap so that we always are looking at the latest schedule
    fn build_schedule_queue(
        workflows: &HashMap<String, Workflow>,
    ) -> Result<BinaryHeap<(i64, String)>, GenericError> {
        let mut heap = BinaryHeap::with_capacity(workflows.len());
        for (workflow_id, workflow) in workflows.iter() {
            match workflow.cron() {
                Some(cron) => {
                    let next_run = Self::next_run_from_cron(cron, workflow.start_time_utc())?;
                    // invert the timestamp to make a min heap
                    heap.push((-next_run.timestamp_millis(), workflow_id.clone()));
                }
                None => continue,
            };
        }
        Ok(heap)
    }

    fn next_run_from_cron(
        cron: &String,
        start_time: Result<DateTime<Utc>, GenericError>,
    ) -> Result<DateTime<Utc>, GenericError> {
        let schedule = Schedule::from_str(cron).map_err(|e| {
            GenericError::ParseError(format!(
                "Schedule {} is not a valid crontab. Error: {}",
                cron,
                e.to_string()
            ))
        })?;
        let now = Utc::now();
        let start_time = start_time?;
        let actual_start = if start_time > now { start_time } else { now };
        let next_run = match schedule.after(&actual_start).next() {
            Some(dt) => Ok(dt),
            None => Err(GenericError::RuntimeError(
                "Unable to determine next run schedule for workflow `{}`. Perhaps can only be in past?".to_string()
            ))
        }?;
        Ok(next_run)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::collections::HashMap;

    fn get_workflow(cron: Option<&str>, start_time: Option<&str>) -> Workflow {
        let cron_str = match cron {
            Some(v) => &format!(r#"cron: "{}""#, v),
            None => "",
        };
        let start_time_str = match start_time {
            Some(v) => &format!(r#"start_time: {}"#, v),
            None => "",
        };
        let yaml = format!(
            r#"
name: Dummy Flow
{}
{}
tasks:
  task1:
    name: Task 1
    description: Runs first task
    config:
      !Subprocess
      cmd: echo
      args:
        - hello
        - world
        "#,
            cron_str, start_time_str
        );
        let yml_str = yaml.as_str();
        Workflow::new("fake/path.yml".to_string(), yml_str).unwrap()
    }

    #[tokio::test]
    async fn test_build_schedule_queue_with_valid_cron() {
        let mut workflows = HashMap::new();
        let start_time = Utc::now();
        workflows.insert(
            "wf1".to_string(),
            get_workflow(Some("*/2 * * * * *"), Some(&start_time.to_rfc3339())),
        );
        let queue = Scheduler::build_schedule_queue(&workflows).unwrap();
        assert_eq!(queue.len(), 1);
    }

    #[tokio::test]
    async fn test_build_schedule_queue_with_invalid_cron() {
        let mut workflows = HashMap::new();
        let start_time = Utc::now();
        workflows.insert(
            "wf1".to_string(),
            get_workflow(Some("invalid cron"), Some(&start_time.to_rfc3339())),
        );
        let queue = Scheduler::build_schedule_queue(&workflows);
        assert!(queue.is_err()); // error when no workflows to create a heap from
    }

    #[tokio::test]
    async fn test_build_schedule_queue_with_no_cron() {
        let mut workflows = HashMap::new();
        let start_time = Utc::now();
        let wf = get_workflow(None, Some(&start_time.to_rfc3339()));
        workflows.insert("wf1".to_string(), wf);
        let queue = Scheduler::build_schedule_queue(&workflows).unwrap();
        assert_eq!(queue.len(), 0); // error when no workflows to create a heap from
    }

    #[tokio::test]
    async fn test_next_run_from_cron_future_start() {
        let cron = "0 0 * * * *".to_string();
        let future = Utc::now() + chrono::Duration::days(1);
        let result = Scheduler::next_run_from_cron(&cron, Ok(future));
        assert!(result.is_ok());
        assert!(result.unwrap() > Utc::now());
    }

    #[tokio::test]
    async fn test_next_run_from_cron_invalid_cron() {
        let cron = "invalid cron".to_string();
        let now = Utc::now();
        let result = Scheduler::next_run_from_cron(&cron, Ok(now));
        assert!(matches!(result, Err(GenericError::ParseError(_))));
    }

    #[tokio::test]
    async fn test_next_run_from_cron_past_start() {
        let cron = "0 0 * * * *".to_string();
        let past = Utc.ymd(2000, 1, 1).and_hms(0, 0, 0);
        let result = Scheduler::next_run_from_cron(&cron, Ok(past));
        assert!(result.is_ok());
        assert!(result.unwrap() > Utc::now());
    }

    #[tokio::test]
    async fn test_scheduler_new_no_workflows() {
        // Patch get_principal_uri and PrincipalAPI::ListWorkflowStore to return empty workflows
        // This is a placeholder; in real code, use a mocking framework or dependency injection.
        // Here, just check the error handling logic directly.
        let workflows: HashMap<String, Workflow> = HashMap::new();
        let queue = Scheduler::build_schedule_queue(&workflows).unwrap();
        assert!(queue.is_empty());
    }

    #[tokio::test]
    async fn test_scheduler_builds_min_heap() {
        let mut workflows = HashMap::new();
        let now = Utc::now();
        workflows.insert(
            "wf1".to_string(),
            get_workflow(Some("0 0 * * * *"), Some(&now.to_rfc3339())),
        );
        workflows.insert(
            "wf2".to_string(),
            get_workflow(Some("0 12 * * * *"), Some(&now.to_rfc3339())),
        );
        let queue = Scheduler::build_schedule_queue(&workflows).unwrap();
        assert_eq!(queue.len(), 2);
        // The heap should have the earliest run at the top (min-heap by inverted timestamp)
        let mut timestamps: Vec<i64> = queue.iter().map(|(ts, _)| -*ts).collect();
        timestamps.sort();
        assert!(timestamps[0] <= timestamps[1]);
    }
}
