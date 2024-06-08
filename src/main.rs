// mod executor;
// mod zookeeper;
mod event;


// // #[tokio::main]
fn main() {
    let callback = |s| println!("GOT FROM ZMQ: {}", s);
    event::run_zmq_listener("0.0.0.0", 5561, callback)
}
