use std::thread;

/// This component is responsible for managing the TaskManager and the Schduler instances
/// and for responding to the requests from the client TUI and REST API
///
use crate::{
    scheduler::Scheduler,
    taskmanager::TaskManager
};
use tokio::sync::mpsc;

enum InstanceType {
    PRINCIPAL,
    AGENT
}
impl InstanceType {
    fn from_str(st: &str) -> Self {
        match st {
            "principal" => Self::PRINCIPAL,
            "PRINCIPAL" => Self::PRINCIPAL,
            "agent" => Self::AGENT,
            "AGENT" => Self::AGENT,
            _o => panic!(
                "Cannot create a Coordinator instance of {}. Must be either PRINCIPAL or AGENT", 
                _o
            )
        }
    }
}

pub struct Coordinator {
    typ: InstanceType,
}
impl Coordinator {
    fn new(typ: &str) -> Self {
        let typ_enum = InstanceType::from_str(typ);
        Self {typ: typ_enum}
    }
    async fn start(
        &self, 
        database_url: Option<String>,
        poll_interval_seconds: i32, 
        max_tm_threads: usize
    ) {
        // TODO: fix issue with scheduler having a std mutex whihc is not safe 
        // across coroutines. 
        match self.typ {
            InstanceType::PRINCIPAL => {
                let tm = TaskManager::new(max_tm_threads);
                let tm_task_queue = tm.task_queue.clone();
                thread::spawn(|| {
                    tokio::spawn(async move {
                        let scheduler = Scheduler::new(database_url, poll_interval_seconds);
                        scheduler.start(tm_task_queue).await
                    })
                });

            },
            InstanceType::AGENT => {
                let tm = TaskManager::new(max_tm_threads);
            }
        }

    }
    // zmq rep/req to respond to current operations
    
    // methods to obtain information from sub component
}