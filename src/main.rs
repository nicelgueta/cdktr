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
async fn spawn_tm(pub_host_cl: String, pub_port: usize,  max_tm_threads: usize) -> tokio::task::JoinHandle<()>{
    tokio::spawn( async move  {
        let mut tm = taskmanager::TaskManager::new(max_tm_threads);
        tm.start(pub_host_cl, pub_port).await
    })
}
async fn spawn_scheduler(
    database_url:Option<String>, 
    poll_interval_seconds:i32, 
    pub_clone:Arc<Mutex<PubSocket>>      
) -> tokio::task::JoinHandle<()>{
    tokio::spawn(async move {
        let mut sched = scheduler::Scheduler::new(
            database_url, 
            poll_interval_seconds
        );
        sched.start(pub_clone).await
    })
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Needs at least one arg of either AGENT or PRINCIPAL");
        return
    };
    let typ = InstanceType::from_str(&args[1]);
    let pub_host = "0.0.0.0".to_string();
    let pub_port = 5561;
    let server_port = 5562;
    let database_url: Option<String> = None;
    let poll_interval_seconds = 2;
    let max_tm_threads = 8;

            

    match typ {
        InstanceType::PRINCIPAL => {
            println!("SERVER: creating publisher on tcp://{pub_host}:{pub_port}");
            let publisher = Arc::new(
                Mutex::new(
                    PubSocket::new()
                )
            );

            println!("SERVER: successfully created publisher");
            // create the scheduler thread that will poll the database 
            // and send task trigger messages to the task manager SUB
            let pub_clone = publisher.clone();
            spawn_scheduler(database_url, poll_interval_seconds, pub_clone).await;

            // start the task manager thread 
            let pub_host_cl = pub_host.clone();
            spawn_tm(pub_host_cl, pub_port, max_tm_threads).await;
            
            let pub_host_cl = pub_host.clone();

            {
                let mut pub_mut = publisher.lock().await;
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

            // start REP/REQ server for principal
            server::principal::start(publisher, &pub_host_cl, server_port).await.expect(
                "CDKTR: Unable to start client server"
            )



        },
        InstanceType::AGENT => {
            // only create the task manager thread since scheduler is not required
            // for AGENT instancess
            loop {
                let pub_host_cl = pub_host.clone();
                let tm_task = spawn_tm(pub_host_cl, pub_port, max_tm_threads).await;
                // start REP/REQ server for agent
                server::agent::start( 
                    &pub_host, 
                    server_port,
                ).await.expect("CDKTR: Unable to start client server");
                println!("SERVER: Loop exited - restarting");
                tm_task.abort();
            }

        }
    };
}