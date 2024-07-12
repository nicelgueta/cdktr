use std::sync::Arc;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex,
};
use zeromq::{PubSocket, Socket};

use crate::{
    models::{traits::EventListener, Task},
    scheduler,
    server::{agent::AgentServer, principal::PrincipalServer, Server},
    taskmanager,
    utils::AsyncQueue,
};

pub enum InstanceType {
    PRINCIPAL,
    AGENT,
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
            ),
        }
    }
}

/// Spawns the TaskManager in a separate coroutine
async fn spawn_tm(
    instance_id: String,
    max_tm_threads: usize,
    task_queue: AsyncQueue<Task>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut tm = taskmanager::TaskManager::new(instance_id, max_tm_threads, task_queue);
        tm.start().await
    })
}

/// Spawns the Scheduler in a separate coroutine
async fn spawn_scheduler(
    database_url: Option<String>,
    poll_interval_seconds: i32,
    task_router_queue: AsyncQueue<Task>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut sched = scheduler::Scheduler::new(database_url, poll_interval_seconds);
        sched.start_listening(task_router_queue).await
    })
}

pub struct Hub {
    publisher: Arc<Mutex<PubSocket>>,
    instance_type: InstanceType,
}

impl Hub {
    pub fn from_instance_type(instance_type: InstanceType) -> Self {
        Self {
            publisher: Arc::new(Mutex::new(PubSocket::new())),
            instance_type,
        }
    }
    pub async fn start(
        &mut self,
        instance_id: String,
        database_url: Option<String>,
        poll_interval_seconds: i32,
        pub_host: String,
        pub_port: usize,
        max_tm_threads: usize,
        server_port: usize,
    ) {
        match self.instance_type {
            InstanceType::PRINCIPAL => {

                // Create the main task queue for the TaskRouter which multiple
                // event listeners can add to
                let task_router_queue = AsyncQueue::new();

                // create the scheduler (which impl the EventListener trait) thread that will poll the database
                // and send task trigger messages to the main receiver that is passed
                // to the task router                
                spawn_scheduler(database_url.clone(), poll_interval_seconds, task_router_queue.clone()).await;

                // // start the task manager thread
                // let pub_host_cl = pub_host.clone();
                // spawn_tm(instance_id, pub_host_cl, pub_port, max_tm_threads).await;

                let pub_host_cl = pub_host.clone();
                // bind the publisher to its TCP port
                {
                    let mut pub_mut = self.publisher.lock().await;
                    pub_mut
                        .bind(&format!("tcp://{}:{}", &pub_host_cl, pub_port))
                        .await
                        .expect(&format!(
                            "Unable to create publisher on {}:{}",
                            &pub_host_cl, pub_port
                        ));
                };

                // create the TaskRouter component which will wait for tasks in its queue
                
                // start REP/REQ server for principal
                let mut principal_server =
                    PrincipalServer::new(self.publisher.clone(), database_url);
                principal_server
                    .start(&pub_host_cl, server_port)
                    .await
                    .expect("CDKTR: Unable to start client server");
            }
            InstanceType::AGENT => {
                // Create the task queue that will be passed to both the task manager and the 
                // server.
                let main_task_queue = AsyncQueue::new();

                let mut agent_server = AgentServer::new(main_task_queue.clone());
                loop {
                    let task_q_cl = main_task_queue.clone();
                    let tm_task = spawn_tm(
                        instance_id.clone(),
                        max_tm_threads,
                        task_q_cl,
                    )
                    .await;
                
                    // start REP/REQ server for agent
                    let agent_loop_exit_code = agent_server
                        .start(&pub_host, server_port)
                        .await
                        .expect("CDKTR: Unable to start client server");
                    println!("SERVER: Loop exited with code {}", agent_loop_exit_code);
                    tm_task.abort();
                    println!("SERVER: Task Manager killed");
                }
            }
        };
    }
}
