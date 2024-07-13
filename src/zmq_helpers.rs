use crate::{models::ZMQArgs, utils::arg_str_to_vec};
use zeromq::{RepSocket, ReqSocket, Socket, ZmqMessage};

impl Into<ZMQArgs> for ZmqMessage {
    fn into(self) -> ZMQArgs {
        let raw_msg = String::try_from(self);
        let raw_string = match raw_msg {
            Ok(s) => s,
            Err(e_str) => e_str.to_string(),
        };
        let arg_vec = arg_str_to_vec(raw_string);
        ZMQArgs::from(arg_vec)
    }
}

/// TODO: add a timeout feature on socket sends
pub async fn get_zmq_req(endpoint_uri: &str) -> ReqSocket {
    let mut req = ReqSocket::new();
    req.connect(endpoint_uri)
        .await
        .expect("Failed to connect to REQ socket");
    req
}

pub async fn get_zmq_rep(endpoint_uri: &str) -> RepSocket {
    let mut rep = RepSocket::new();
    rep.bind(endpoint_uri)
        .await
        .expect("Failed to connect to REQ socket");
    rep
}

pub fn get_agent_tcp_uri(agent_id: &String) -> String {
    // TODO: Change to use datastore instead
    // since these Ids will change
    return format!("tcp://0.0.0.0:{}", agent_id);
}
