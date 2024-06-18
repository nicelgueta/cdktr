mod executor;
mod taskmanager;
mod interfaces;
mod scheduler;
mod db;
mod server;

use taskmanager::TaskManager;

#[tokio::main]
async fn main() {
    let mut tm = TaskManager::new(2);
    tm.start("0.0.0.0".to_string(), 5561).await
}
