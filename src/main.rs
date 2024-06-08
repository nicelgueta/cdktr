mod executor;
mod zookeeper;

use std::{time, thread};
use zookeeper::Zookeeper;

// // #[tokio::main]
fn main() {
    let mut zk = Zookeeper::new(2);

    zk.run_in_executor("python".to_string(), vec!["s.py".to_string(), "3".to_string()]).unwrap();

    zk.run_in_executor("python".to_string(), vec!["s.py".to_string(), "6".to_string()]).unwrap();

    let second = time::Duration::from_secs(1);
    thread::sleep(second);
    zk.wait_on_threads();
}
