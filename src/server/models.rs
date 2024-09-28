use zeromq::ZmqMessage;

use crate::models::ZMQArgs;

#[derive(Debug)]
pub enum RepReqError {
    ParseError(String),
    Unprocessable(String),
    ServerError(String),
}
impl RepReqError {
    pub fn to_string(&self) -> String {
        match self {
            Self::ParseError(pl) => format!("PARSE ERROR: {}", pl),
            Self::Unprocessable(pl) => format!("UNPROCESSABLE: {}", pl),
            Self::ServerError(pl) => format!("SERVER ERROR: {}", pl),
        }
    }
}

/// A message that is returned to the client REQ socket.
#[derive(PartialEq, Debug)]
pub enum ClientResponseMessage {
    ClientError(String),
    ServerError(String),
    Unprocessable(String),
    Pong,
    Success,
    SuccessWithPayload(String),
}

macro_rules! get_payload {
    ($args:expr, $variant:ident) => {
        match $args.next() {
            Some(v) => Self::$variant(v),
            None => Self::ServerError(format!(
                "{} missing first argument: payload",
                stringify!($variant)
            )),
        }
    };
}

impl From<ZmqMessage> for ClientResponseMessage {
    fn from(value: ZmqMessage) -> Self {
        let mut args: ZMQArgs = value.into();
        let msg_type = if let Some(v) = args.next() {
            v
        } else {
            return Self::ClientError("Cannot work with an empty message".to_string());
        };
        match msg_type.as_str() {
            "CLIENTERROR" => get_payload!(&mut args, ClientError),
            "SERVERERROR" => get_payload!(&mut args, ServerError),
            "UNPROC" => get_payload!(&mut args, Unprocessable),
            "PONG" => Self::Pong,
            "OK" => Self::Success,
            "SUCCESS" => get_payload!(&mut args, SuccessWithPayload),
            mt => Self::ClientError(format!("Unrecognised message type: {}", mt)),
        }
    }
}

impl Into<String> for ClientResponseMessage {
    fn into(self) -> String {
        match self {
            Self::ClientError(payload) => format!("CLIENTERROR|{payload}"),
            Self::Pong => "PONG".to_string(),
            Self::Success => "OK".to_string(),
            Self::SuccessWithPayload(payload) => format!("SUCCESS|{payload}"),
            Self::ServerError(payload) => format!("SERVERERROR|{payload}"),
            Self::Unprocessable(payload) => format!("UNPROC|{payload}"),
        }
    }
}

impl Into<ZmqMessage> for ClientResponseMessage {
    fn into(self) -> ZmqMessage {
        let msg: String = self.into();
        ZmqMessage::from(msg)
    }
}
