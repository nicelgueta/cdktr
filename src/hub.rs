use crate::db::models::ScheduledTask;
use tokio::sync::{mpsc::{Receiver, Sender, channel}, Mutex};
use zeromq::{Socket, PubSocket};
use std::sync::Arc;

use crate::{
    taskmanager, scheduler, server, interfaces::Task
};

pub enum InstanceType {
    PRINCIPAL,
    AGENT
}
impl InstanceType {
    pub fn from_str(st: &str) -> Self {
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
async fn spawn_tm(instance_id: String, pub_host_cl: String, pub_port: usize,  max_tm_threads: usize) -> tokio::task::JoinHandle<()>{
    tokio::spawn( async move  {
        let mut tm = taskmanager::TaskManager::new(instance_id, max_tm_threads);
        tm.start(pub_host_cl, pub_port).await
    })
}
async fn spawn_scheduler(
    database_url:Option<String>, 
    poll_interval_seconds:i32, 
    sender: Sender<ScheduledTask>  
) -> tokio::task::JoinHandle<()>{
    tokio::spawn(async move {
        let mut sched = scheduler::Scheduler::new(
            database_url, 
            poll_interval_seconds
        );
        sched.start(sender).await
    })
}

pub struct Hub {
    publisher: Arc<Mutex<PubSocket>>,
    instance_type: InstanceType,
    tx: Sender<Task>,
    rx: Receiver<Task>
}

impl Hub {
    pub fn from_instance_type(instance_type: InstanceType) -> Self {
        let (tx, rx) = channel::<Task>(32);
        Self {
            publisher: Arc::new(Mutex::new(PubSocket::new())),
            instance_type,
            tx, 
            rx
        }
    }
    pub async fn start(
        &mut self, 
        instance_id: String,
        database_url:Option<String>,
        poll_interval_seconds:i32,
        pub_host: String,
        pub_port: usize,
        max_tm_threads: usize,
        server_port: usize

    ) {
        match self.instance_type {
            InstanceType::PRINCIPAL => {

                // create the scheduler thread that will poll the database 
                // and send task trigger messages to the main receiver that is passed
                // to the task router
                
                spawn_scheduler(database_url, poll_interval_seconds, self.tx).await;
        
                // start the task manager thread 
                let pub_host_cl = pub_host.clone();
                spawn_tm(instance_id, pub_host_cl, pub_port, max_tm_threads).await;
                
                let pub_host_cl = pub_host.clone();
        
                {
                    let mut pub_mut = self.publisher.lock().await;
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
                server::principal::start(
                    self.publisher, 
                    &pub_host_cl, 
                    server_port
                ).await.expect(
                    "CDKTR: Unable to start client server"
                )
            },
            InstanceType::AGENT => {
                // only create the task manager thread since scheduler is not required
                // for AGENT instancess
                loop {
                    let pub_host_cl = pub_host.clone();
                    let tm_task = spawn_tm(instance_id.clone(), pub_host_cl, pub_port, max_tm_threads).await;
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
}          