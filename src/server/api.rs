
use zeromq::ZmqMessage;

pub struct ClientConversionError;

pub enum ClientResponseMessage {
    InvalidMessageType,
    Pong,
}

impl Into<ZmqMessage> for ClientResponseMessage {
    fn into(self) -> ZmqMessage {
        let s = match self {
            Self::InvalidMessageType => "InvalidRequest: Unrecognised message type",
            Self::Pong => "PONG"
        };
        ZmqMessage::from(s)
    }
}

pub enum ClientRequestMessage {
    Ping,
    // GetTasks,
}
impl ClientRequestMessage {
    fn from_zmq_str(s: &str) -> Result<Self, ClientConversionError> {
        let parsed_s: Vec<&str> = s.split("|").collect();
        let msg_type = parsed_s[0];
        let args: Vec<&str> = parsed_s[1..].into();
        match msg_type {
            // "GET_TASKS" => Ok(Self::GetTasks),
            "PING" => Ok(Self::Ping),
            _ => Err(ClientConversionError)
        }
    }
}
impl TryFrom<ZmqMessage> for ClientRequestMessage {
    type Error = ClientConversionError;
    fn try_from(value: ZmqMessage) -> Result<Self, Self::Error> {
        let zmq_msg_s = String::try_from(value).expect(
            "Unable to convert ZMQ Client message to String"
        );
        Self::from_zmq_str(&zmq_msg_s)
    }
}
