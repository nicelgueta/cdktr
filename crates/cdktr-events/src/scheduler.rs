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
    next_peek: i64,
}

#[async_trait]
impl EventListener<Task> for Scheduler {
    async fn start_listening(&mut self) -> Result<(), GenericError> {
        loop {
            let mut ms_to_wait = self.next_peek - Utc::now().timestamp_millis();
            ms_to_wait = if ms_to_wait < 0 { 0 } else { ms_to_wait };
            let time_to_wait = Duration::from_millis(ms_to_wait as u64);

            // put this component to sleep until the allotted time
            sleep(time_to_wait).await;

            let workflow_id = self.schedule_priority_queue.pop().unwrap().1;
            self.run_workflow(&workflow_id).await?;

            // add the next run of the same workflow back to priority queue
            let workflow = self.workflows.get(&workflow_id).unwrap();
            match workflow.cron() {
                Some(cron) => {
                    let next_run = Self::next_run_from_cron(cron, workflow.start_time_utc())?;
                    // invert the timestamp to make a min heap
                    self.schedule_priority_queue
                        .push((-next_run.timestamp_millis(), workflow_id));
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
        let workflows = match response {
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
        let schedule_priority_queue = Self::build_schedule_queue(&workflows)?;
        if schedule_priority_queue.is_empty() {
            return Err(GenericError::NoDataException(
                "No workflows have valid schedules. Scheduler cannot run".to_string(),
            ));
        };
        let next_peek = schedule_priority_queue.peek().unwrap().0;
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
