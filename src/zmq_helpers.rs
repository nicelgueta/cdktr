use zeromq::{RepSocket, ReqSocket, Socket, ZmqMessage};
use crate::{models::ZMQArgs, utils::arg_str_to_vec};

impl Into<ZMQArgs> for ZmqMessage {
    fn into(self) -> ZMQArgs {
        let raw_msg = String::try_from(self);
        let raw_string = match raw_msg {
            Ok(s) => s,
            Err(e_str) => e_str.to_string()
        };
        let arg_vec = arg_str_to_vec(raw_string);
        ZMQArgs::from(arg_vec)
    }
}

pub async fn get_zmq_req(endpoint_uri: &str) -> ReqSocket {
    let mut req = ReqSocket::new();
    req.connect(endpoint_uri).await.expect("Failed to connect to REQ socket");
    req
}

pub async fn get_zmq_rep(endpoint_uri: &str) -> RepSocket {
    let mut rep = RepSocket::new();
    rep.bind(endpoint_uri).await.expect("Failed to connect to REQ socket");
    rep
}
