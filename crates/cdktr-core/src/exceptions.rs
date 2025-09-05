use std::{fmt, num::ParseIntError};

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
    ZMQError(String),
    ParseError(String),
    RuntimeError(String),
    WorkflowError(String),
    NoDataException(String), // APIError(String),
    DBError(String),
    DBQueryStatementError(String),
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
            Self::WorkflowError(s) => format!("WorkflowError: {}", s.clone()),
            Self::ZMQError(s) => format!("ZMQError: {}", s.clone()),
            Self::DBError(s) => format!("DBError: {}", s.clone()),
            Self::DBQueryStatementError(s) => format!("DBError: {}", s.clone()),
            // Self::APIError(s) => s.clone(),
        }
    }
}
impl fmt::Display for GenericError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl Into<GenericError> for zeromq::ZmqError {
    fn into(self) -> GenericError {
        GenericError::ZMQError(self.to_string())
    }
}

impl Into<GenericError> for ParseIntError {
    fn into(self) -> GenericError {
        GenericError::ParseError(format!(
            "Value is not a valid integer. Original error: {}",
            self.to_string()
        ))
    }
}

pub fn cdktr_result<T, E: Into<GenericError>>(r: Result<T, E>) -> Result<T, GenericError> {
    match r {
        Ok(t) => Ok(t),
        Err(e) => Err(e.into()),
    }
}
