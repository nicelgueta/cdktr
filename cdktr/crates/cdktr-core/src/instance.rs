use log::{debug, warn};
use std::{sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::sleep};

use crate::{
    client::PrincipalClient,
    db::get_connection,
    models::Task,
    server::{principal::PrincipalServer, traits::Server},
    taskmanager,
    utils::{data_structures::AsyncQueue, get_instance_id},
    zmq_helpers::{get_server_tcp_uri, DEFAULT_TIMEOUT},
};

const POLL_INTERVAL_MS: u64 = 1;

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

async fn start_fetch_task_loop(
    mut task_queue: AsyncQueue<Task>,
    principal_client: &PrincipalClient,
    principal_uri: &str,
) {
    loop {
        loop {
            sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
            if let Err(e) = principal_client
                .process_fetch_task(&mut task_queue, principal_uri, DEFAULT_TIMEOUT)
                .await
            {
                let msg = e.to_string();
                debug!("Failed get success on fetching task: {}", msg);
                break;
            }
        }
        // broken out of loop owing to timeout so we need to re-register with the principal
        warn!("Attempting to reconnect to principal @ {}", principal_uri);
        if let Err(_e) = principal_client
            .register_with_principal(principal_uri)
            .await
        {
            break;
        }
    }
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
        max_tm_tasks: Option<usize>,
    ) {
        let instance_id = get_instance_id(&instance_host, instance_port);
        match self.instance_type {
            InstanceType::PRINCIPAL => {
                let db_cnxn = Arc::new(Mutex::new(get_connection(database_url.as_deref())));
                debug!(
                    "Created db connection to {}",
                    database_url.unwrap_or(String::from(":memory:"))
                );

                let mut principal_server =
                    PrincipalServer::new(db_cnxn.clone(), instance_id.clone());

                // start REP/REQ server loop for principal
                principal_server
                    .start(&principal_host, instance_port)
                    .await
                    .expect("CDKTR: Unable to start client server");
            }
            InstanceType::AGENT => {
                let principal_uri = get_server_tcp_uri(&principal_host, principal_port);
                let principal_client = PrincipalClient::new(instance_id.clone());
                let _ = principal_client
                    .register_with_principal(&principal_uri)
                    .await;

                // create the task manager that handles the task execution threads
                let task_queue = AsyncQueue::new();
                let max_tasks = max_tm_tasks.expect(
                    "Agent cannot be instantiatied without specifying max number of concurrent tasks it can handle."
                );
                spawn_tm(instance_id, max_tasks, task_queue.clone()).await;

                debug!("Starting agent heartbeat loop");
                start_fetch_task_loop(task_queue, &principal_client, &principal_uri).await;
            }
        };
    }
}
