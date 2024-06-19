mod executor;
mod taskmanager;
mod interfaces;
mod db;
mod scheduler;
mod server;

use tokio::sync::Mutex;
use zeromq::{Socket, PubSocket};
use std::env;
use std::sync::Arc;
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
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Needs at least one arg of either AGENT or PRINCIPAL");
        return
    };
    let typ = InstanceType::from_str(&args[1]);
    let pub_host = "0.0.0.0".to_string();
    let pub_port = 5561;
    let server_port = 5563;
    let database_url = None;
    let poll_interval_seconds = 2;
    let max_tm_threads = 2;

            

    match typ {
        InstanceType::PRINCIPAL => {
            println!("SERVER: creating publisher on tcp://{pub_host}:{pub_port}");
            let publisher = Arc::new(Mutex::new(PubSocket::new()));

            println!("SERVER: successfully created publisher");
            // create the scheduler thread that will poll the database 
            // and send task trigger messages to the task manager SUB
            let pub_clone = publisher.clone();
            thread::spawn(move ||{
                let rt = Builder::new_current_thread()
                    .enable_time()
                    .enable_io()
                    .build()
                    .unwrap();
                rt.block_on(async move {
                    let mut sched = scheduler::Scheduler::new(database_url, poll_interval_seconds);
                    sched.start(pub_clone).await
                })
            });

            // start the task manager thread 
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
            let pub_host_cl = pub_host.clone();
            let pub_clone = publisher.clone();
            // start REP/REQ server for principal
            let rt = Builder::new_current_thread()
                .enable_time()
                .enable_io()
                .build()
                .unwrap();
            rt.block_on(async move {
                {
                    let mut pub_mut = pub_clone.lock().await;
                    pub_mut
                        .bind(
                            &format!(
                                "tcp://{}:{}", &pub_host_cl, pub_port
                            )
                        )
                        .await
                        .expect(&format!(
                            "Unable to create publisher on {}:{}", &pub_host_cl, pub_port
                        ));
                };
                server::principal::start(publisher, &pub_host_cl, server_port).await.expect(
                    "CDKTR: Unable to start client server"
                )
            });



        },
        InstanceType::AGENT => {
            // only create the task manager thread since scheduler is not required
            // for AGENT instancess
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
            // start REP/REQ server for principal
            let rt = Builder::new_current_thread()
                .enable_time()
                .enable_io()
                .build()
                .unwrap();
            rt.block_on(async move {
                server::agent::start( &pub_host, server_port).await.expect(
                    "CDKTR: Unable to start client server"
                )
            });
        }
    };
}