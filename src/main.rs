mod db;
mod exceptions;
mod executors;
mod hub;
mod macros;
mod models;
mod scheduler;
mod server;
mod task_router;
mod taskmanager;
mod utils;
mod zmq_helpers;

use hub::{Hub, InstanceType};
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Needs at least arg (1) of either AGENT or PRINCIPAL and (2) PORT");
        return;
    };
    let typ = InstanceType::from_str(&args[1]);
    let pub_host = "0.0.0.0".to_string();
    let pub_port = 5561;
    let server_port: usize = args[2].parse().expect("PORT must be a valid number");
    let database_url: Option<String> = None;
    let poll_interval_seconds = 2;
    let max_tm_threads = 8;

    let instance_id = server_port.to_string();

    let mut hub = Hub::from_instance_type(typ);

    // begin main app loop
    hub.start(
        instance_id,
        database_url,
        poll_interval_seconds,
        pub_host,
        pub_port,
        max_tm_threads,
        server_port,
    )
    .await
}
