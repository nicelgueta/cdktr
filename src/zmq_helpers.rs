use zeromq::ZmqMessage;
use crate::{models::ZMQArgs, utils::arg_str_to_vec};
use crate::server::models::ClientResponseMessage;

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

impl Into<ZmqMessage> for ClientResponseMessage {
    fn into(self) -> ZmqMessage {
        let s = match self {
            Self::ClientError(payload) => format!("CLIENTERROR|{payload}"),
            Self::Pong => "PONG".to_string(),
            Self::Success => "SUCCESS".to_string(),
            Self::SuccessWithPayload(payload) => format!("SUCCESS|{payload}"),
            Self::Heartbeat(pub_id) => format!("HEARTBEAT|{pub_id}"),
            Self::ServerError(payload) => format!("SERVERERROR|{payload}"),
            Self::Unprocessable(payload) => format!("UNPROC|{payload}"),
        };
        ZmqMessage::from(s)
    }
}