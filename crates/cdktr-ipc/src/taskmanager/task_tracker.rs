use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
};

use cdktr_core::exceptions::GenericError;
use cdktr_workflow::Workflow;
use std::sync::Mutex;

pub trait TaskTracker
where
    Self: Sized,
{
    fn from_workflow(workflow: &Workflow) -> Result<Self, GenericError>;
    fn get_next_task(&mut self) -> Option<String>;
    fn mark_complete(&mut self, task_id: &str);
    fn is_empty(&self) -> bool;
}

/// Struct required after topologically sorting the workflow tasks
/// to manage execution dependency.
/// Given for example the DAG with prec/succ edges:
/// A -> B
/// A -> C
/// B -> D
/// C -> D
/// C -> E
///
/// A valid top sort could be [A, B, C, D, E]. Another could be: [A, C, B, E, D]
/// 'A' is the first node and can be executed immediately. Once this has been completed
/// 'B' and 'C' can be executed in parallel. Our top sort doesn't keep track of the individual
/// dependencies however, so if 'B' ran for a while after 'C' completes, even though 'E' is good to go
/// there's no real way of knowing just from the top sort alone, so we have to actively keep
/// track of the individual dependencies in order for every task to run at the right time.
struct BaseTaskTracker {
    dep_graph: VecDeque<(String, HashSet<String>)>,
    ready_q: VecDeque<String>,
}
impl TaskTracker for BaseTaskTracker {
    fn from_workflow(workflow: &Workflow) -> Result<Self, GenericError> {
        workflow.validate()?;
        let mut dep_graph = VecDeque::new();
        let mut ready_q = VecDeque::new();
        for (task_id, task) in workflow.get_tasks() {
            if let Some(prec_task_ids) = task.get_dependencies() {
                if prec_task_ids.is_empty() {
                    ready_q.push_back(task_id.clone());
                    continue;
                };
                let mut dep_set = HashSet::new();
                for ptask_id in prec_task_ids {
                    dep_set.insert(ptask_id);
                }
                dep_graph.push_back((task_id.clone(), dep_set));
            } else {
                ready_q.push_back(task_id.clone());
            };
        }
        Ok(Self { dep_graph, ready_q })
    }

    fn get_next_task(&mut self) -> Option<String> {
        self.ready_q.pop_front()
    }

    fn mark_complete(&mut self, task_id: &str) {
        let mut new_dep_graph = VecDeque::new();
        //TODO. bit inefficient to copy it everytime - maybe look for a better way
        let old_graph = self.dep_graph.clone();
        for (asso_task_id, mut dep_set) in old_graph {
            let removed_from_set = dep_set.remove(task_id);
            if removed_from_set && dep_set.len() == 0 {
                // removed all deps - this one is ready
                self.ready_q.push_back(asso_task_id);
            } else {
                new_dep_graph.push_back((asso_task_id, dep_set));
            }
        }
        self.dep_graph = new_dep_graph;
    }

    fn is_empty(&self) -> bool {
        self.dep_graph.is_empty() && self.ready_q.is_empty()
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

    fn mark_complete(&mut self, task_id: &str) {
        (*self.tt.lock().unwrap()).mark_complete(task_id)
    }

    fn is_empty(&self) -> bool {
        (*self.tt.lock().unwrap()).is_empty()
    }
}
