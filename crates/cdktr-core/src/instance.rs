use log::{debug, info, warn};
use std::{sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::sleep};

use crate::{
    db::get_connection,
    server::{principal::PrincipalServer, traits::Server},
    taskmanager,
    utils::get_instance_id,
    zmq_helpers::get_server_tcp_uri,
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
        info!("{}", e.to_string())
    };
}

/// Starts the main principal loop
pub async fn start_principal(
    instance_host: String,
    instance_port: usize,
    database_url: Option<String>,
) {
    let instance_id = get_instance_id(&instance_host, instance_port);
    let db_cnxn = Arc::new(Mutex::new(get_connection(database_url.as_deref())));
    debug!(
        "Created db connection to {}",
        database_url.unwrap_or(String::from(":memory:"))
    );

    let mut principal_server = PrincipalServer::new(db_cnxn.clone(), instance_id.clone());

    // start REP/REQ server loop for principal
    principal_server
        .start(&instance_host, instance_port)
        .await
        .expect("CDKTR: Unable to start client server");

    std::process::exit(1); // loop has broken
}
