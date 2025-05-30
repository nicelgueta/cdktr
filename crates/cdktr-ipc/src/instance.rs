use std::{env, time::Duration};

use crate::{
    server::{principal::PrincipalServer, traits::Server},
    taskmanager,
};
use cdktr_core::{get_cdktr_setting, zmq_helpers::get_server_tcp_uri};
use cdktr_workflow::WorkflowStore;
use log::{debug, error, info, warn};
use tokio::time::sleep;

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
pub async fn start_principal(instance_host: String, instance_port: usize, instance_id: String) {
    let workflows = WorkflowStore::from_dir("./example_cdktr_tasks")
        .await
        .expect("Failed to load workflow store on load"); // TODO: remove hardcode
    info!("Loaded {} workflows into store", workflows.count().await);
    let mut principal_server = PrincipalServer::new(instance_id.clone(), workflows.clone());

    // start workflow refresh loop
    tokio::spawn(async move { workflowstore_refresh_loop(workflows).await });

    // start REP/REQ server loop for principal
    principal_server
        .start(&instance_host, instance_port)
        .await
        .expect("CDKTR: Unable to start client server");

    std::process::exit(1); // loop has broken
}

/// refreshes the principal instance with the latest workflows
/// from the main directory. Controlled with the
/// CDKTR_WORKFLOW_DIR_REFRESH_FREQUENCY_S setting
async fn workflowstore_refresh_loop(mut workflows: WorkflowStore) {
    let interval = Duration::from_secs(get_cdktr_setting!(
        CDKTR_WORKFLOW_DIR_REFRESH_FREQUENCY_S,
        usize
    ) as u64);
    loop {
        sleep(interval).await;
        workflows.refresh_workflows().await
    }
}
