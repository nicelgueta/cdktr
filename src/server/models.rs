#[derive(Debug)]
pub enum RepReqError {
    ParseError(String),
    Unprocessable(String),
    ServerError(String),
}
impl RepReqError {
    pub fn new(typ: usize, msg: String) -> Self {
        match typ {
            1 => Self::ParseError(msg),
            2 => Self::Unprocessable(msg),
            3 => Self::ServerError(msg),
            _ => Self::ServerError(format!("Unhandled exception. Code {typ}")),
        }
    }
    pub fn to_string(&self) -> String {
        match self {
            Self::ParseError(pl) => pl.clone(),
            Self::Unprocessable(pl) => pl.clone(),
            Self::ServerError(pl) => pl.clone(),
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
    Heartbeat(String),
}
