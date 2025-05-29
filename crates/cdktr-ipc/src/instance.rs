use cdktr_core::{utils::get_instance_id, zmq_helpers::get_server_tcp_uri};
use log::{debug, error};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    server::{principal::PrincipalServer, traits::Server},
    taskmanager,
};

/// Starts the main agent loop
pub async fn start_agent(
    instance_id: String,
    principal_host: String,
    principal_port: usize,
    max_tm_tasks: usize,
) {
    let principal_uri = get_server_tcp_uri(&principal_host, principal_port);
    let mut tm = taskmanager::TaskManager::new(instance_id, max_tm_tasks, principal_uri).await;
    let loop_res = tm.start().await;
    if let Err(e) = loop_res {
        error!("{}", e.to_string());
        std::process::exit(1);
    };
}

/// Starts the main principal loop
pub async fn start_principal(instance_host: String, instance_port: usize) {
    let instance_id = get_instance_id(&instance_host, instance_port);

    let mut principal_server = PrincipalServer::new(instance_id.clone());

    // start REP/REQ server loop for principal
    principal_server
        .start(&instance_host, instance_port)
        .await
        .expect("CDKTR: Unable to start client server");

    std::process::exit(1); // loop has broken
}
