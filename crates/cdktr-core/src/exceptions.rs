#[derive(Debug, PartialEq)]
pub enum ZMQParseError {
    ParseError(String),
    // InvalidMessageType,
    InvalidTaskType,
}
impl ZMQParseError {
    pub fn to_string(&self) -> String {
        match self {
            Self::ParseError(msg) => format!("ParseError: {msg}"),
            // Self::InvalidMessageType => String::from("Invalid message type"),
            Self::InvalidTaskType => String::from("Invalid task type"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum GenericError {
    MissingAgents,
    TimeoutError,
    ZMQParseError(ZMQParseError),
    ParseError(String),
    RuntimeError(String),
    NoDataException(String), // APIError(String),
}
impl GenericError {
    pub fn to_string(&self) -> String {
        match self {
            Self::MissingAgents => String::from("Missing agents: No running agents found"),
            Self::TimeoutError => String::from("Call timed out"),
            Self::ZMQParseError(zmq_e) => format!("ZMQ Error: {}", zmq_e.to_string()),
            Self::RuntimeError(s) => format!("Runtime Error: {}", s.clone()),
            Self::NoDataException(s) => format!("NoDataException: {}", s.clone()),
            Self::ParseError(s) => format!("ParseError: {}", s.clone()),
            // Self::APIError(s) => s.clone(),
        }
    }
}
