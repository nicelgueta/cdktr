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
    NetworkError(String),
}

impl ClientResponseMessage {
    pub fn to_string(&self) -> String {
        match self {
            Self::Pong => "PONG".to_string(),
            Self::Success => "OK".to_string(),
            Self::SuccessWithPayload(payload) => format!("SUCCESS|{payload}"),

            Self::ClientError(payload) => format!("CLIENTERROR|{payload}"),
            Self::ServerError(payload) => format!("SERVERERROR|{payload}"),
            Self::Unprocessable(payload) => format!("UNPROC|{payload}"),
            Self::NetworkError(payload) => format!("NETWORKERROR|{payload}"),
        }
    }

    /// Convenience method used to unpack a client message payload into just the string without
    /// the initial token that's used to denote the message type. If the message does not have a
    /// payload then just an empty string is returned
    pub fn payload(&self) -> String {
        match self {
            Self::Pong => "".to_string(),
            Self::Success => "".to_string(),
            Self::SuccessWithPayload(pl) => pl.clone(),

            Self::ClientError(pl) => pl.clone(),
            Self::ServerError(pl) => pl.clone(),
            Self::Unprocessable(pl) => pl.clone(),
            Self::NetworkError(pl) => pl.clone(),
        }
    }
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
            "CLIENTERROR" => Self::ClientError(args.to_string()),
            "SERVERERROR" => Self::ServerError(args.to_string()),
            "UNPROC" => Self::Unprocessable(args.to_string()),
            "PONG" => Self::Pong,
            "OK" => Self::Success,
            "SUCCESS" => Self::SuccessWithPayload(args.to_string()),
            mt => Self::ClientError(format!("Unrecognised message type: {}", mt)),
        }
    }
}

impl Into<String> for ClientResponseMessage {
    fn into(self) -> String {
        self.to_string()
    }
}

impl Into<ZmqMessage> for ClientResponseMessage {
    fn into(self) -> ZmqMessage {
        let msg: String = self.into();
        ZmqMessage::from(msg)
    }
}

#[cfg(test)]
mod tests {
    use zeromq::ZmqMessage;

    use super::ClientResponseMessage;

    #[test]
    fn test_client_message_success_payload() {
        let zmq_m = ZmqMessage::from("SUCCESS|SOME random payload|with|other_args");
        let cli_msg = ClientResponseMessage::from(zmq_m);
        assert_eq!(
            cli_msg.payload(),
            "SOME random payload|with|other_args".to_string()
        )
    }

    #[test]
    fn test_client_message_success_payload_direct_match() {
        let zmq_m = ZmqMessage::from("SUCCESS|SOME random payload|with|other_args");
        let cli_msg = ClientResponseMessage::from(zmq_m);
        match cli_msg {
            ClientResponseMessage::SuccessWithPayload(pl) => {
                assert_eq!(pl, "SOME random payload|with|other_args".to_string())
            }
            _ => panic!("Expected only success payload for this test"),
        }
    }
}
