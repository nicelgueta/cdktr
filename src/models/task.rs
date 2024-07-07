use std::collections::VecDeque;

use super::exceptions::ZMQParseError;
use serde::Deserialize;
use zeromq::ZmqMessage;
use super::ZMQMessageType;
use crate::utils::arg_str_to_vec;

use crate::executors::ProcessTask;

/// A Task is the encapsulation provided for single unit of work defined and utilised
/// by difference components of the system. On the ZMQ sockets, it's encoded as a 
/// pipe-delimited string with the first token being `TASKDEF` and the second being the
/// uppercase representation of a TaskType enum to determine the type. 
/// A Task type defines the types of tasks supported by cdktr for execution. 
/// The value of each enum must define the struct configuration for each task
#[derive(Debug, PartialEq, Clone, Deserialize)]
pub enum Task {
    Process(ProcessTask)
}

impl TryFrom<VecDeque<String>> for Task {
    type Error = ZMQParseError;
    fn try_from(mut zmq_msg_v: VecDeque<String>) -> Result<Self, Self::Error>{
        if zmq_msg_v.len() == 0 {
            return Err(ZMQParseError::InvalidTaskType)
        };
        let typ_tok = zmq_msg_v.pop_front().unwrap();
        match typ_tok.as_str() {
            "PROCESS" => {
                if zmq_msg_v.len() < 1 {
                    Err(ZMQParseError::ParseError(
                        "Missing tokens for PROCESS msg. Expected tokens COMMAND and ARGS".to_string()
                    ))
                } else {
                    let command = zmq_msg_v.pop_front().unwrap();
                    let args = if zmq_msg_v.len() < 1 {
                        None
                    } else {
                        Some(zmq_msg_v.into())
                    };
                    Ok(Task::Process(
                        ProcessTask {
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

impl TryFrom<ZmqMessage> for Task {
    type Error = ZMQParseError;
    fn try_from(value: ZmqMessage) -> Result<Self, Self::Error> {
        let zmq_str = String::try_from(value);
        if let Err(e) = zmq_str {
            return Err(ZMQParseError::ParseError(e.to_string()))
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
                Ok(Task::try_from(zmq_msg_v)?)
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