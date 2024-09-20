use std::{sync::Arc, time::Duration};
use log::{debug, error, info, trace, warn};
use tokio::{sync::Mutex, time::sleep};
use zeromq::{SocketRecv, SocketSend};

use crate::{
    db::get_connection,
    exceptions::GenericError,
    models::{traits::EventListener, AgentPriorityQueue, Task},
    scheduler,
    server::{
        agent::AgentServer,
        models::ClientResponseMessage,
        principal::{PrincipalAPI, PrincipalServer},
        Server,
    },
    task_router::TaskRouter,
    taskmanager,
    utils::{get_instance_id, AsyncQueue},
    zmq_helpers::{get_server_tcp_uri, DEFAULT_TIMEOUT, send_recv_with_timeout},
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
    pub fn to_string(&self) -> String {
        match self {
            Self::AGENT => String::from("AGENT"),
            Self::PRINCIPAL => String::from("PRINCIPAL"),
        }
    }
}

/// Spawns the TaskManager in a separate coroutine
async fn spawn_tm(
    instance_id: String,
    max_tm_tasks: usize,
    task_queue: AsyncQueue<Task>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut tm = taskmanager::TaskManager::new(instance_id, max_tm_tasks, task_queue);
        tm.start().await
    })
}

/// Spawns the Scheduler in a separate coroutine
async fn spawn_scheduler(
    db_cnxn: Arc<Mutex<diesel::SqliteConnection>>,
    poll_interval_seconds: i32,
    task_router_queue: AsyncQueue<Task>,
) -> tokio::task::JoinHandle<()> {
    debug!("Spawning scheduler");
    let handle = tokio::spawn(async move {
        let mut sched = scheduler::Scheduler::new(db_cnxn, poll_interval_seconds);
        sched.start_listening(task_router_queue).await
    });
    debug!("Scheduler spawned");
    handle
}

async fn spawn_task_router(
    task_router_queue: AsyncQueue<Task>,
    live_agents: AgentPriorityQueue,
) -> tokio::task::JoinHandle<()> {
    debug!("Spawning Task Router");
    let handle = tokio::spawn(async move {
        let mut task_router = TaskRouter::new(task_router_queue, live_agents);
        task_router.start().await
    });
    debug!("Task Router spawned");
    handle
}

async fn spawn_principal_heartbeat(
    instance_id: String,
    principal_uri: String,
    max_tm_tasks: usize,
) {
    debug!("Spawning agent heartbeat loop");
    tokio::spawn(async move {
        loop {
            loop {
                sleep(Duration::from_millis(1000)).await;
                let msg = PrincipalAPI::Ping;
                trace!("Pinging principal @ {} with msg: {}", &principal_uri, msg.to_string());
                let resp_res = send_recv_with_timeout(
                    principal_uri.clone(),
                    msg.into(),
                    DEFAULT_TIMEOUT
                ).await;
                match resp_res {
                    Ok(zmq_msg) => {
                        let msg: String = ClientResponseMessage::from(zmq_msg).into();
                        trace!("Principal response: {}", msg)
                    },
                    Err(e) => match e {
                            GenericError::TimeoutError => {
                                error!("Agent heartbeat timed out pinging principal");
                                break
                            },
                            _ => panic!("Unspecified error in principal heartbeat"),
                        }
                    }
                
            }
            // broken out of loop owing to timeout so we need to re-register with the principal
            warn!("Attempting to reconnect to principal @ {}", &principal_uri);
            let msg = PrincipalAPI::RegisterAgent(instance_id.clone(), max_tm_tasks);
            let resp_res = send_recv_with_timeout(
                principal_uri.clone(), msg.into(), DEFAULT_TIMEOUT
            ).await;
            match resp_res {
                Ok(zmq_message) => {
                    let cli_msg = ClientResponseMessage::from(zmq_message);
                    match cli_msg {
                        ClientResponseMessage::Success => info!("Successfully reconnected to principal"),
                        e => {
                            let msg_str: String = e.into();
                            error!(
                                "Established connection to principal but got unexpected message: {msg_str}", 
                            );
                            break; // kill the loop for good
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to re-register with principal. Got error {}",
                        e.to_string()
                    );
                    break;
                }
            };
        }
    });
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
        instance_host: String,
        instance_port: usize,
        principal_host: String,
        principal_port: usize,
        database_url: Option<String>,
        poll_interval_seconds: i32,
        max_tm_tasks: usize,
    ) {
        let instance_id = get_instance_id(&instance_host, instance_port);
        match self.instance_type {
            InstanceType::PRINCIPAL => {
                let db_cnxn = Arc::new(Mutex::new(get_connection(database_url.as_deref())));
                debug!("Created db connection to {}", database_url.unwrap_or(String::from(":memory:")));

                // create the priority queue of agent meta that will be used by the server
                // and task router
                let live_agents = AgentPriorityQueue::new();
                let mut principal_server = PrincipalServer::new(
                    db_cnxn.clone(),
                    instance_id.clone(),
                    Some(live_agents.clone()),
                );

                // Create the main task queue for the TaskRouter which multiple
                // event listeners can add to
                let task_router_queue = AsyncQueue::new();

                // create the scheduler (which impl the EventListener trait) thread that will poll the database
                // and send task trigger messages to the main receiver that is passed
                // to the task router
                spawn_scheduler(db_cnxn, poll_interval_seconds, task_router_queue.clone()).await;

                // create the TaskRouter component which will wait for tasks in its queue
                spawn_task_router(task_router_queue, live_agents).await;

                // start REP/REQ server loop for principal
                principal_server
                    .start(&principal_host, instance_port)
                    .await
                    .expect("CDKTR: Unable to start client server");
            }
            InstanceType::AGENT => {
                // Create the task queue that will be passed to both the task manager and the
                // server.
                let main_task_queue = AsyncQueue::new();

                let mut agent_server =
                    AgentServer::new(instance_id.clone(), main_task_queue.clone());
                let principal_uri = get_server_tcp_uri(&principal_host, principal_port);
                agent_server
                    .register_with_principal(&principal_uri, max_tm_tasks)
                    .await;

                // start heartbeat coroutine loop to check if reconnecting is required
                spawn_principal_heartbeat(instance_id.clone(), principal_uri.clone(), max_tm_tasks)
                    .await;

                loop {
                    let task_q_cl = main_task_queue.clone();
                    let tm_task = spawn_tm(instance_id.clone(), max_tm_tasks, task_q_cl).await;

                    // start REP/REQ server for agent
                    let agent_loop_exit_code = agent_server
                        .start(&instance_host, instance_port)
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
