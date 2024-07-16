use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use crate::{
    db::get_connection,
    models::{traits::EventListener, AgentConfig, Task},
    scheduler,
    server::{agent::AgentServer, principal::PrincipalServer, Server},
    task_router::TaskRouter,
    taskmanager,
    utils::AsyncQueue,
    zmq_helpers::get_agent_tcp_uri,
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
    db_cnxn: Arc<Mutex<diesel::SqliteConnection>>,
    poll_interval_seconds: i32,
    task_router_queue: AsyncQueue<Task>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut sched = scheduler::Scheduler::new(db_cnxn, poll_interval_seconds);
        sched.start_listening(task_router_queue).await
    })
}

async fn spawn_task_router(
    task_router_queue: AsyncQueue<Task>,
    live_agents: Arc<Mutex<HashMap<String, AgentConfig>>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut task_router = TaskRouter::new(task_router_queue, live_agents);
        task_router.start().await
    })
}

pub struct Hub {
    instance_type: InstanceType,
}

impl Hub {
    pub fn from_instance_type(instance_type: InstanceType) -> Self {
        Self { instance_type }
    }
    pub async fn start(
        &mut self,
        instance_id: String,
        database_url: Option<String>,
        poll_interval_seconds: i32,
        pub_host: String,
        max_tm_threads: usize,
        server_port: usize,
    ) {
        match self.instance_type {
            InstanceType::PRINCIPAL => {
                let db_cnxn = Arc::new(Mutex::new(get_connection(database_url.as_deref())));
                let mut principal_server = PrincipalServer::new(db_cnxn.clone(), instance_id.clone());

                // Create the main task queue for the TaskRouter which multiple
                // event listeners can add to
                let task_router_queue = AsyncQueue::new();

                // create the scheduler (which impl the EventListener trait) thread that will poll the database
                // and send task trigger messages to the main receiver that is passed
                // to the task router
                spawn_scheduler(db_cnxn, poll_interval_seconds, task_router_queue.clone()).await;
                let live_agents_arc = principal_server.get_live_agents_ptr();

                // create the TaskRouter component which will wait for tasks in its queue
                spawn_task_router(task_router_queue, live_agents_arc).await;

                // start REP/REQ server loop for principal
                principal_server
                    .start(&pub_host, server_port)
                    .await
                    .expect("CDKTR: Unable to start client server");
            }
            InstanceType::AGENT => {
                // Create the task queue that will be passed to both the task manager and the
                // server.
                let main_task_queue = AsyncQueue::new();

                let mut agent_server =
                    AgentServer::new(instance_id.clone(), main_task_queue.clone());

                // TODO: currently hardcoded principal - change to a CLI arg
                agent_server
                    .register_with_principal(&get_agent_tcp_uri(&"5562".to_string()))
                    .await;
                loop {
                    let task_q_cl = main_task_queue.clone();
                    let tm_task = spawn_tm(instance_id.clone(), max_tm_threads, task_q_cl).await;

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
