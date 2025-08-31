use std::{
    env::{self, home_dir},
    time::Duration,
};

use crate::{
    log_manager::{
        manager::LogManager,
        persister::{start_listener, start_persistence_loop},
    },
    server::{principal::PrincipalServer, traits::Server},
    taskmanager,
};
use cdktr_core::{
    exceptions::GenericError, get_cdktr_setting, utils::data_structures::AsyncQueue,
    zmq_helpers::get_server_tcp_uri,
};
use cdktr_db::DBClient;
use cdktr_events::start_scheduler;
use cdktr_workflow::WorkflowStore;
use log::{error, info, warn};
use tokio::{task::JoinSet, time::sleep};

/// Starts the main agent loop
pub async fn start_agent(
    instance_id: String,
    principal_host: String,
    principal_port: usize,
    max_concurrent_workflows: usize,
) {
    let principal_uri = get_server_tcp_uri(&principal_host, principal_port);
    let mut tm =
        taskmanager::TaskManager::new(instance_id, max_concurrent_workflows, principal_uri).await;
    let loop_res = tm.start().await;
    if let Err(e) = loop_res {
        error!("{}", e.to_string());
        std::process::exit(1);
    };
}

/// Starts the main principal loop
pub async fn start_principal(
    instance_host: String,
    instance_port: usize,
    instance_id: String,
) -> Result<(), GenericError> {
    let db_path = get_cdktr_setting!(CDKTR_DB_PATH);
    let db_path_str = if db_path.contains("$HOME") {
        db_path.replace(
            "$HOME",
            home_dir()
                .expect("Unable to determine user home directory")
                .to_str()
                .expect("Unable to determine user home directory"),
        )
    } else {
        db_path
    };
    let db_client =
        DBClient::new(Some(&db_path_str)).expect("Failed to create DB client on start up");
    let workflows = WorkflowStore::from_dir(get_cdktr_setting!(CDKTR_WORKFLOW_DIR).as_str())
        .await
        .expect("Failed to load workflow store on load");
    info!("Loaded {} workflows into store", workflows.count().await);
    let mut principal_server =
        PrincipalServer::new(instance_id.clone(), workflows.clone(), db_client.clone());

    let mut m_joined: JoinSet<Result<(), GenericError>> = JoinSet::new();

    // start workflow refresh loop
    m_joined.spawn(async move {
        admin_refresh_loop(workflows).await;
        Ok::<(), GenericError>(())
    });

    // start logs manager
    m_joined.spawn(async move {
        LogManager::new().await?.start().await;
        Ok::<(), GenericError>(())
    });

    // logs persistence to db
    let logs_queue = AsyncQueue::new();
    let lq_clone = logs_queue.clone();
    let db_clone = db_client.clone();
    // start logs persistence listener
    m_joined.spawn(async move { start_listener(lq_clone).await });
    // start logs persistence db job
    m_joined.spawn(async move {
        start_persistence_loop(db_clone, logs_queue).await;
        Ok::<(), GenericError>(())
    });

    // start REP/REQ server loop for principal
    m_joined.spawn(async move {
        principal_server
            .start(&instance_host, instance_port)
            .await
            .expect("CDKTR: Unable to start client server");
        Ok::<(), GenericError>(())
    });

    // start scheduler
    m_joined.spawn(async move {
        // give the rest of the app 2 seconds to start up before activating schedules
        sleep(Duration::from_millis(2_000)).await;
        start_scheduler().await
    });

    let _ = m_joined.join_all().await;
    std::process::exit(1); // loop has broken
}

/// Runs regular refresh tasks within the principal like persisting the task queue
/// and refreshing workflows from the main directory.
async fn admin_refresh_loop(mut workflows: WorkflowStore) {
    let interval = Duration::from_secs(get_cdktr_setting!(
        CDKTR_WORKFLOW_DIR_REFRESH_FREQUENCY_S,
        usize
    ) as u64);
    loop {
        sleep(interval).await;
        workflows.refresh_workflows().await
    }
}
