mod executor;
mod taskmanager;
mod interfaces;
mod db;
mod scheduler;
mod server;

use std::thread;
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
                "Cannot create a Server instance of {}. Must be either PRINCIPAL or AGENT", 
                _o
            )
        }
    }
}

// #[tokio::main]
fn main() {
    let typ = InstanceType::from_str("PRINCIPAL");
    let pub_host = "0.0.0.0".to_string();
    let pub_port = 5561;
    let database_url = None;
    let poll_interval_seconds = 2;
    let max_tm_threads = 2;

    match typ {
        InstanceType::PRINCIPAL => {
            let pub_host_cl = pub_host.clone();
            thread::spawn(move ||{
                let rt = Builder::new_current_thread()
                    .enable_time()
                    .enable_io()
                    .build()
                    .unwrap();
                rt.block_on(async move {
                    let mut sched = scheduler::Scheduler::new(database_url, poll_interval_seconds);
                    sched.start(pub_host_cl, pub_port).await
                })
            });
            let pub_host_cl = pub_host.clone();
            thread::spawn( move || {
                let rt = Builder::new_current_thread()
                    .enable_time()
                    .enable_io()
                    .build()
                    .unwrap();
                rt.block_on(async move {
                    // let scheduler spin up before running tm
                    let mut tm = taskmanager::TaskManager::new(max_tm_threads);
                    tm.start(pub_host_cl, pub_port).await
                });
            });

        },
        InstanceType::AGENT => {
            let pub_host_cl = pub_host.clone();
            thread::spawn(move || {
                let rt = Builder::new_current_thread()
                    .enable_time()
                    .enable_io()
                    .build()
                    .unwrap();
                rt.block_on(async move {
                    let mut tm = taskmanager::TaskManager::new(max_tm_threads);
                    tm.start(pub_host_cl, pub_port).await
                })
            });
        }
    };
    let rt = Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .unwrap();
    rt.block_on(async move {
        let serv = server::Server{};
        serv.start(&pub_host, pub_port+1).await.expect(
            "CDKTR: Unable to start client server"
        )
    });
}