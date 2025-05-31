use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
};

use cdktr_core::exceptions::GenericError;
use cdktr_workflow::{WorkFlowDAG, Workflow};
use std::sync::Mutex;

pub trait TaskTracker
where
    Self: Sized,
{
    fn from_workflow(workflow: &Workflow) -> Result<Self, GenericError>;
    fn get_next_task(&mut self) -> Option<String>;
    fn mark_success(&mut self, task_id: &str) -> Result<(), GenericError>;
    fn mark_failed(&mut self, task_id: &str) -> Result<(), GenericError>;
    fn is_finished(&self) -> bool;
}

/// Struct required to manage execution dependency.
/// required to track individual dependencies and outcomes
/// so that in the event of failure, tasks dependent on the failure
/// are skipped and those are not can continue.
struct BaseTaskTracker {
    dag: WorkFlowDAG,
    ready_q: VecDeque<String>,
    failed_stack: Vec<String>,
    skipped_stack: Vec<String>,
    success_stack: Vec<String>,
    processed_count: usize,
}
impl TaskTracker for BaseTaskTracker {
    fn from_workflow(workflow: &Workflow) -> Result<Self, GenericError> {
        workflow.validate()?;
        let dag = workflow.get_dag().clone();
        let ready_q = dag.get_first_tasks().into();
        Ok(Self {
            dag: dag,
            ready_q,
            failed_stack: Vec::new(),
            skipped_stack: Vec::new(),
            success_stack: Vec::new(),
            processed_count: 0,
        })
    }

    fn get_next_task(&mut self) -> Option<String> {
        self.ready_q.pop_front()
    }

    fn mark_success(&mut self, task_id: &str) -> Result<(), GenericError> {
        for next_task_id in self.dag.get_dependents(task_id)? {
            self.ready_q.push_back(next_task_id.clone());
        }
        self.success_stack.push(task_id.to_string());
        self.processed_count += 1;
        Ok(())
    }

    fn mark_failed(&mut self, task_id: &str) -> Result<(), GenericError> {
        self.failed_stack.push(task_id.to_string());
        self.processed_count += 1;
        let mut skip_q: VecDeque<&String> = VecDeque::new();
        for next_task_id in self.dag.get_dependents(task_id)? {
            skip_q.push_back(next_task_id);
        }
        while !skip_q.is_empty() {
            let task_to_skip = skip_q.pop_front().unwrap();
            self.skipped_stack.push(task_to_skip.clone());
            self.processed_count += 1;
            for next_task_id in self.dag.get_dependents(task_to_skip)? {
                skip_q.push_back(next_task_id);
            }
        }
        Ok(())
    }

    fn is_finished(&self) -> bool {
        self.dag.node_count() == self.processed_count
    }
}

#[derive(Clone)]
pub struct ThreadSafeTaskTracker {
    tt: Arc<Mutex<BaseTaskTracker>>,
}
impl TaskTracker for ThreadSafeTaskTracker {
    fn from_workflow(workflow: &Workflow) -> Result<Self, GenericError> {
        Ok(Self {
            tt: Arc::new(Mutex::new(BaseTaskTracker::from_workflow(workflow)?)),
        })
    }

    fn get_next_task(&mut self) -> Option<String> {
        (*self.tt.lock().unwrap()).get_next_task()
    }

    fn mark_success(&mut self, task_id: &str) -> Result<(), GenericError> {
        (*self.tt.lock().unwrap()).mark_success(task_id)
    }

    fn mark_failed(&mut self, task_id: &str) -> Result<(), GenericError> {
        (*self.tt.lock().unwrap()).mark_failed(task_id)
    }

    fn is_finished(&self) -> bool {
        (*self.tt.lock().unwrap()).is_finished()
    }
}
