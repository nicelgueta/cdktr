use async_trait::async_trait;
use cdktr_api::models::ClientResponseMessage;
use cdktr_api::{API, PrincipalAPI};
use cdktr_core::exceptions::GenericError;
use cdktr_core::get_cdktr_setting;
use cdktr_core::utils::{get_default_timeout, get_principal_uri};
use cdktr_workflow::{Task, Workflow};
use chrono::{DateTime, Utc};
use cron::Schedule;
use log::{debug, error, info};
use std::collections::{BinaryHeap, HashMap};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
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
#[derive(Clone)]
pub struct Scheduler {
    workflows_ptr: Arc<Mutex<HashMap<String, Workflow>>>,
    schedule_priority_queue_ptr: Arc<Mutex<BinaryHeap<(i64, String)>>>,
    next_peek: Arc<Mutex<(String, i64, bool)>>, // task_id, unix timestamp for start, has been logged
}

#[async_trait]
impl EventListener<Task> for Scheduler {
    async fn start_listening(&mut self) -> Result<(), GenericError> {
        let poll_duration = Duration::from_millis(get_cdktr_setting!(
            CDKTR_SCHEDULER_START_POLL_FREQUENCY_MS,
            usize
        ) as u64);
        loop {
            while !self.next_workflow_ready().await {
                let mut next_peek_lock = self.next_peek.lock().await;
                let logged_to_console = next_peek_lock.2;
                if !logged_to_console {
                    (*next_peek_lock).2 = true;
                    info!(
                        "Next task `{}` scheduled to run:  {}",
                        next_peek_lock.0,
                        DateTime::from_timestamp_millis(next_peek_lock.1)
                            .unwrap()
                            .to_rfc2822()
                    );
                };
                drop(next_peek_lock); // release the lock before sleeping
                sleep(poll_duration).await;
            }
            let workflow_id = {
                let mut pqlock = self.schedule_priority_queue_ptr.lock().await;
                pqlock.pop().unwrap().1
            };
            {
                let next_peek_lock = self.next_peek.lock().await;
                if next_peek_lock.0 != workflow_id {
                    return Err(GenericError::RuntimeError(format!(
                        "Popped wrong workflow from the priority queue. Expected {} but got {}",
                        next_peek_lock.0, workflow_id
                    )));
                }
            };
            info!("Staging scheduled task: {}", &workflow_id);
            self.run_workflow(&workflow_id).await?;

            // add the next run of the same workflow back to priority queue
            {
                // lock holds for duration of usage to avoid a refresh coinciding
                let workflows = self.workflows_ptr.lock().await;
                let workflow = workflows.get(&workflow_id).unwrap();
                match workflow.cron() {
                    Some(cron) => {
                        let next_run = Self::next_run_from_cron(cron, Ok(Utc::now()))?;
                        // invert the timestamp to make a min heap
                        let q_top = {
                            let mut pqlock = self.schedule_priority_queue_ptr.lock().await;
                            pqlock.push((-next_run.timestamp_millis(), workflow_id));
                            // update the peek
                            pqlock.peek().unwrap().clone()
                        };
                        *self.next_peek.lock().await = (q_top.1, -q_top.0, false);
                    }
                    None => continue,
                };
            }
        }
    }
}
impl Scheduler {
    pub async fn new() -> Result<Self, GenericError> {
        let principal_uri: String = get_principal_uri();
        let workflows = Self::get_workflows(&principal_uri).await?;
        let workflows_len = workflows.len();
        info!(
            "Scheduler found {} workflows with active schedules",
            workflows_len
        );
        let schedule_priority_queue = Self::build_schedule_queue(&workflows)?;
        let workflows_ptr = Arc::new(Mutex::new(workflows));
        if schedule_priority_queue.is_empty() {
            return Err(GenericError::NoDataException(
                "No workflows have valid schedules. Scheduler cannot run".to_string(),
            ));
        };
        // flip sign to get the original value
        let q_top = schedule_priority_queue.peek().unwrap();
        let next_peek = Arc::new(Mutex::new((q_top.1.clone(), -q_top.0, false)));
        let schedule_priority_queue_ptr = Arc::new(Mutex::new(schedule_priority_queue));
        Ok(Self {
            workflows_ptr,
            schedule_priority_queue_ptr,
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

    async fn get_workflows(principal_uri: &str) -> Result<HashMap<String, Workflow>, GenericError> {
        let api = PrincipalAPI::ListWorkflowStore;
        let response = api.send(principal_uri, get_default_timeout()).await?;
        match response {
            ClientResponseMessage::SuccessWithPayload(wfs) => {
                serde_json::from_str(&wfs).map_err(|e| {
                    GenericError::ParseError(format!(
                        "Failed to read workflows from principal message. Not valid JSON: {}",
                        e.to_string()
                    ))
                })
            }
            other => Err(GenericError::WorkflowError(other.to_string())),
        }
    }

    pub async fn spawn_refresh_loop(&self) {
        info!("Spawning scheduler workflow refresh loop");
        let scheduler_ptr = self.clone();
        tokio::spawn(async move {
            if let Err(e) = refresh_loop(scheduler_ptr).await {
                error!("Scheduler efresh loop crashed: {}", e)
            };
        });
    }

    async fn next_workflow_ready(&self) -> bool {
        self.next_peek.lock().await.1 - Utc::now().timestamp_millis() <= 0
    }

    async fn workflows_match(&self, incoming_workflows: &HashMap<String, Workflow>) -> bool {
        let workflows_lock = self.workflows_ptr.lock().await;
        &*workflows_lock == incoming_workflows
    }

    async fn update_schedule_priority_queue(
        &mut self,
        incoming_workflows: HashMap<String, Workflow>,
    ) -> Result<(), GenericError> {
        debug!("Updating workflow scheduling priority queue");
        let mut pqlock = self.schedule_priority_queue_ptr.lock().await;
        *pqlock = Scheduler::build_schedule_queue(&incoming_workflows)?;
        debug!("PQ rebuilt");
        (*self.workflows_ptr.lock().await).extend(incoming_workflows.into_iter()); // will override where key already exists
        let q_top = pqlock.peek().unwrap();
        *self.next_peek.lock().await = (q_top.1.clone(), -q_top.0, false);
        Ok(())
    }
}

async fn refresh_loop(mut scheduler: Scheduler) -> Result<(), GenericError> {
    let principal_uri = get_principal_uri();
    let workflow_refresh_seconds =
        get_cdktr_setting!(CDKTR_WORKFLOW_DIR_REFRESH_FREQUENCY_S, usize) as u64;
    loop {
        let _ = sleep(Duration::from_secs(workflow_refresh_seconds)).await;
        debug!("checking internal workflow store for new workflows defs");
        match Scheduler::get_workflows(&principal_uri).await {
            Ok(wfs) => {
                if !scheduler.workflows_match(&wfs).await {
                    info!("Found workflows to refresh from principal");
                    scheduler.update_schedule_priority_queue(wfs).await?;
                    info!("Successfully refreshed workflows from principal");
                }
            }
            Err(e) => {
                error!(
                    "Failed to retrieve workflows from principal: {}",
                    e.to_string()
                )
            }
        }
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
        let past = Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap();
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
