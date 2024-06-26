use std::collections::VecDeque;

use exceptions::ZMQParseError;
use zeromq::ZmqMessage;

use crate::utils::arg_str_to_vec;

pub mod task_types;

mod exceptions {
    #[derive(Debug, PartialEq)]
    pub enum ZMQParseError {
        ParseError(String),
        InvalidMessageType,
        InvalidTaskType
    }
}

#[derive(Debug, PartialEq)]
pub enum FlowExecutionResult {
    SUCCESS,
    CRASHED(String),
    // ABORTED(String),
    // FAILURE(String),
}
impl FlowExecutionResult {
    pub fn to_string(self) -> String {
        match self {
            Self::CRASHED(v) => v,
            _ => "".to_string()
            // Self::ABORTED(v) => v,
            // Self::FAILURE(v) => v,
        }
    }
}

/// ZMQMessageType enum defines the possible messages that can travel on the 
/// on the wire between different components and externally. The first token
/// in a ZMQString is matched against this enum to determine whether a message
/// appears to be a supported message based on this token. It is up to the actual
/// implementation of the ZMQEncodable itself to determine whether the rest of the string
/// is valid or not for the message type.
pub enum ZMQMessageType {
    TaskDef,
}
impl ZMQMessageType {
    pub fn new(token: &str) -> Result<Self, exceptions::ZMQParseError> {
        match token {
            "TASKDEF" => Ok(Self::TaskDef),
            _ => Err(exceptions::ZMQParseError::InvalidMessageType)
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TaskDef => "TASKDEF"
        }
    }
}

/// A Task is the encapsulation provided for single unit of work defined and utilised
/// by difference components of the system. On the ZMQ sockets, it's encoded as a 
/// pipe-delimited string with the first token being `TASKDEF` and the second being the
/// uppercase representation of a TaskType enum to determine the type. 
/// A Task type defines the types of tasks supported by cdktr for execution. 
/// The value of each enum must define the struct configuration for each task
#[derive(Debug, PartialEq)]
pub enum Task {
    Process(task_types::ProcessTask)
}

impl TryFrom<ZmqMessage> for Task {
    type Error = ZMQParseError;
    fn try_from(value: ZmqMessage) -> Result<Self, Self::Error> {
        let zmq_str = String::try_from(value);
        if let Err(e) = zmq_str {
            return Err(exceptions::ZMQParseError::ParseError(e.to_string()))
        };
        let mut zmq_msg_v = VecDeque::from(arg_str_to_vec(&zmq_str.unwrap()));
        if zmq_msg_v.len() == 0 {
            return Err(ZMQParseError::ParseError(
                "Empty message - no valid tokens".to_string()
            ));
        };
        let msg_type_token = zmq_msg_v.pop_front().unwrap();
        let msg_type = ZMQMessageType::new(&msg_type_token)?;
        match msg_type {
            ZMQMessageType::TaskDef => {
                if zmq_msg_v.len() == 0 {
                    return Err(exceptions::ZMQParseError::InvalidTaskType)
                };
                let typ_tok = zmq_msg_v.pop_front().unwrap();
                match typ_tok.as_str() {
                    "PROCESS" => {
                        if zmq_msg_v.len() < 1 {
                            Err(exceptions::ZMQParseError::ParseError(
                                "Missing tokens for PROCESS msg. Expected tokens COMMAND and ARGS".to_string()
                            ))
                        } else {
                            let command = zmq_msg_v.pop_front().unwrap();
                            let args = if zmq_msg_v.len() < 1 {
                                None
                            } else {
                                Some(zmq_msg_v.into())
                            };
                            Ok(Self::Process(
                                task_types::ProcessTask {
                                    command,
                                    args
                                }
                            ))
                        }
                    },
                    _ => Err(ZMQParseError::InvalidTaskType)
                }
            }
        }
    }
}
impl TryInto<ZmqMessage> for Task {
    type Error = ZMQParseError;
    fn try_into(self) -> Result<ZmqMessage, Self::Error> {
        match self {
            Self::Process(pt) => {
                let mut tokens = vec![
                    "TASKDEF".to_string(),
                    "PROCESS".to_string()
                ];
                tokens.push(pt.command.clone());
                if let Some(args) = &pt.args {
                    for arg in args {
                        tokens.push(arg.clone())
                    }
                };
                Ok(ZmqMessage::from(tokens.join("|")))
            }
        }
    }
}


pub mod traits {
    use tokio::sync::mpsc::Sender;
    use zeromq::ZmqMessage;

    use super::FlowExecutionResult;
    use core::future::Future;

    pub trait ZMQEncodable: From<ZmqMessage> + Into<ZmqMessage> {}

    pub trait Executor {
        fn new(command: &str, args: Option<Vec<String>>) -> Self ;
        fn run(self, tx: Sender<String>) -> impl Future<Output = FlowExecutionResult> ;
    }

}


#[cfg(test)]
mod tests {
    use zeromq::ZmqMessage;
    use super::*;

    #[test]
    fn zmq_message_type_taskdef() {
        assert!(ZMQMessageType::new("TASKDEF").is_ok());
    }

    #[test]
    fn zmq_message_type_invalid() {
        assert!(ZMQMessageType::new("invalidinvalid").is_err());
    }


    #[test]
    fn create_process_task_from_zmq_message() {
        let msg = ZmqMessage::from("TASKDEF|PROCESS|ls");
        assert!(Task::try_from(msg).is_ok());
    }

    #[test]
    fn create_process_task_from_zmq_msg_with_args() {
        let msg = ZmqMessage::from("TASKDEF|PROCESS|ls|thisdir");
        assert!(Task::try_from(msg).is_ok());
    }

    #[test]
    fn create_process_task_from_zmq_string_missing_command() {
        let msg = ZmqMessage::from("TASKDEF|PROCESS");
        assert!(Task::try_from(msg).is_err());
    }

}