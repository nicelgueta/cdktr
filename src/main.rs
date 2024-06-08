
mod executor;
mod zookeeper;
mod event;

use zookeeper::Zookeeper;

// // #[tokio::main]
fn main() {
    let  mut zk = Zookeeper::new(2);
    zk.main_event_loop("0.0.0.0", 5561)
}
