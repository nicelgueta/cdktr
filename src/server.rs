use std::{thread::{self, sleep}, time::Duration};

/// This component is responsible for managing the TaskManager and the Schduler instances
/// and for responding to the requests from the client TUI and REST API
///
use crate::{
    scheduler::Scheduler,
    taskmanager::TaskManager
};
use tokio::runtime::Builder;

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
    pub fn new(typ: &str) -> Self {
        let typ_enum = InstanceType::from_str(typ);
        Self {typ: typ_enum}
    }
    pub fn start(
        &self, 
        pub_host: String,
        pub_port: usize,
        max_tm_threads: usize,
        database_url: Option<String>,
        poll_interval_seconds: i32, 
    ) {
        match self.typ {
            InstanceType::PRINCIPAL => {
                let pub_host_cl = pub_host.clone();
                thread::spawn(move ||{
                    let rt = Builder::new_current_thread()
                        .enable_time()
                        .enable_io()
                        .build()
                        .unwrap();
                    rt.block_on(async move {
                        let mut sched = Scheduler::new(database_url, poll_interval_seconds);
                        sched.start(pub_host_cl, pub_port).await
                    })
                });
                // allow time for sched to boot
                sleep(Duration::from_secs(2));
                thread::spawn( move || {
                    let rt = Builder::new_current_thread()
                        .enable_time()
                        .enable_io()
                        .build()
                        .unwrap();
                    rt.block_on(async move {
                        // let scheduler spin up before running tm
                        let mut tm = TaskManager::new(max_tm_threads);
                        tm.start(pub_host, pub_port).await
                    });
                });

            },
            InstanceType::AGENT => {
                thread::spawn(move || {
                    let rt = Builder::new_current_thread()
                        .enable_time()
                        .enable_io()
                        .build()
                        .unwrap();
                    rt.block_on(async move {
                        let mut tm = TaskManager::new(max_tm_threads);
                        tm.start(pub_host, pub_port).await
                    })
                });
            }
        };
        // enter REP/REQ loop
        println!("Simulating entering the server loop");
        sleep(Duration::from_secs(10))

    }
    // zmq rep/req to respond to current operations
    
    // methods to obtain information from sub component
}