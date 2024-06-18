mod executor;
mod taskmanager;
mod interfaces;
mod db;
mod scheduler;
mod server;
use server::Coordinator;
#[tokio::main]
async fn main() {
    let master = Coordinator::new("PRINCIPAL");
    master.start("0.0.0.0".to_string(), 5561, 2, None, 2);
}