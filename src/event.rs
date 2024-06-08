
pub fn run_zmq_listener<F>(host: &str, port: usize, mut callback: F)
where F: FnMut(String)
{
    let context = zmq::Context::new();
    let subscriber = context.socket(zmq::SUB).unwrap();
    subscriber
        .connect(&format!("tcp://{}:{}", host, &port.to_string()))
        .unwrap();
    subscriber
        .set_subscribe(b"")
        .unwrap();
    
    // TODO: 
    // sync with publisher
    // let sync_uri = format!("tcp://{}:{}", host, &(port+1).to_string());
    // println!("{}", &sync_uri);
    // let syncclient = context.socket(zmq::REQ).unwrap();
    // syncclient
    //     .connect(&sync_uri)
    //     .expect("failed connect syncclient");

    // syncclient.send("", 0).expect("failed sending sync request");
    // println!("Awaiting sync from publisher");

    // syncclient.recv_msg(0).expect("failed receiving sync reply");
    // println!("Synced with pub");

    // let mut msg = zmq::Message::new();
    println!("Started ZMQ Event Listener on tcp://{}:{}", host, &port.to_string());
    loop {
        let msg = subscriber
            .recv_string(0)
            .expect("failed to get message")
            .expect("Failed to decode to utf8");

        callback(msg);
    }
}