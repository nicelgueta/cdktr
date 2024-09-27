#[derive(Debug, PartialEq)]
pub enum ZMQParseError {
    ParseError(String),
    InvalidMessageType,
    InvalidTaskType,
}
impl ZMQParseError {
    pub fn to_string(&self) -> String {
        match self {
            Self::ParseError(msg) => format!("ParseError: {msg}"),
            Self::InvalidMessageType => String::from("Invalid message type"),
            Self::InvalidTaskType => String::from("Invalid task type"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum GenericError {
    MissingAgents,
    TimeoutError,
    ZMQParseError(ZMQParseError),
    RuntimeError(String),
    APIError(String)
}
impl GenericError {
    pub fn to_string(&self) -> String {
        match self {
            Self::MissingAgents => String::from("No running agents found"),
            Self::TimeoutError => String::from("Call timed out"),
            Self::ZMQParseError(zmq_e) => zmq_e.to_string(),
            Self::RuntimeError(s) => s.clone(),
            Self::APIError(s) => s.clone()
        }
    }
}
