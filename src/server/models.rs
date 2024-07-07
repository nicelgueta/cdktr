

#[derive(Debug)]
pub struct ClientConversionError {
    pub msg: String
}
impl ClientConversionError {
    pub fn new(msg: String) -> Self {
        ClientConversionError {msg}
    }
    pub fn to_string(&self) -> String {
        self.msg.clone()
    }
}


#[derive(PartialEq, Debug)]
pub enum ClientResponseMessage {
    InvalidMessageType,
    ClientError(String),
    ServerError(String),
    Pong,
    Success,
    SuccessWithPayload(String),
    Heartbeat(String)
}

