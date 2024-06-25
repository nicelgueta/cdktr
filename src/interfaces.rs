use exceptions::ZMQParseError;


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
enum ZMQMessageType {
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

/// ZMQString is a thin wrapper around a normal string that is passed on the write by ZMQ
/// to ensure it is formatted in a way that can easily be decoded by components 
/// and publish and receive messages.
/// The struct defines the message type and the subsequent tokens that were found on the 
/// message. This separation is used to ensure that the message type is determined by
/// the consumer and the tokens consumed with the context of the message known.
struct ZMQString {
    msg_type: ZMQMessageType,
    tokens: Vec<String>
}
impl ZMQString {
    pub fn new(msg_type: ZMQMessageType, tokens: Vec<String>) -> Self {
        Self {
            msg_type, tokens
        }
    }
    pub fn from_raw(raw: String) -> Result<Self, exceptions::ZMQParseError> {
        let tokens: Vec<String> = raw.split("|").map(
            |x|x.to_string()
        ).collect();
        if tokens.len() == 0 {
            return Err(exceptions::ZMQParseError::InvalidMessageType)
        };
        // validate msg type
        let msg_type = ZMQMessageType::new(&tokens[0])?;
        Ok(
            Self {
                tokens: tokens[1..].into(),
                msg_type
            }
        )
    }
    pub fn to_raw(&self) -> String {
        let mut s = String::new();
        let mt = self.msg_type.as_str();
        s.push_str(mt);
        s.push_str(&self.tokens.join("|"));
        s
    }
    pub fn tokens(&self) -> &Vec<String> {
        &self.tokens
    }
}

#[derive(Debug,PartialEq)]
struct ProcessTask {
    pub command: String,
    pub args: Option<Vec<String>>
}
/// A Task is the encapsulation provided for single unit of work defined and utilised
/// by difference components of the system. On the ZMQ sockets, it's encoded as a 
/// pipe-delimited string with the first token being `TASKDEF` and the second being the
/// uppercase representation of a TaskType enum to determine the type. 
/// A Task type defines the types of tasks supported by cdktr for execution. 
/// The value of each enum must define the struct configuration for each task
#[derive(Debug, PartialEq)]
enum Task {
    Process(ProcessTask)
}
impl traits::ZMQEncodable for Task {
    fn from_zmq_str(s: ZMQString) -> Result<Self, exceptions::ZMQParseError> 
    where Self: Sized {
        match s.msg_type {
            ZMQMessageType::TaskDef => {
                if s.tokens.len() == 0 {
                    return Err(exceptions::ZMQParseError::InvalidTaskType)
                };
                let typ_tok = s.tokens[0].as_str();
                match typ_tok {
                    "PROCESS" => {
                        if s.tokens[1..].len() < 1 {
                            Err(exceptions::ZMQParseError::ParseError(
                                "Missing tokens for PROCESS msg. Expected tokens COMMAND and ARGS".to_string()
                            ))
                        } else {
                            let args = if s.tokens[2..].len() < 1 {
                                None
                            } else {
                                Some(s.tokens[2..].into())
                            };
                            let command = s.tokens[1].clone();
                            Ok(Self::Process(
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
    }
    fn to_zmq_string(&self) -> ZMQString {
        match self {
            Self::Process(pt) => {
                let mut tokens = vec!["PROCESS".to_string()];
                tokens.push(pt.command.clone());
                if let Some(args) = &pt.args {
                    for arg in args {
                        tokens.push(arg.clone())
                    }
                };
                ZMQString::new(
                    ZMQMessageType::TaskDef, tokens
                )
            }
        }
    }

}

pub mod traits {
    use tokio::sync::mpsc::Sender;

    use super::{FlowExecutionResult, ZMQString, exceptions};
    use core::future::Future;

    pub trait ZMQEncodable {
        fn to_zmq_string(&self) -> ZMQString;
        fn from_zmq_str(s: ZMQString) -> Result<Self, exceptions::ZMQParseError> 
        where Self: Sized;
    }

    pub trait Executor {
        fn new(command: &str, args: Option<Vec<String>>) -> Self ;
        fn run(self, tx: Sender<String>) -> impl Future<Output = FlowExecutionResult> ;
    }

}


#[cfg(test)]
mod tests {
    use traits::ZMQEncodable;

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
    fn zmq_string_task_new() {
        ZMQString::new(
            ZMQMessageType::TaskDef, vec!["ls".to_string()]
        );
    }

    #[test]
    fn zmq_string_task_from_raw() {
        assert!(ZMQString::from_raw(
            "TASKDEF|UNDEFINED".to_string()
        ).is_ok())
    }

    #[test]
    fn zmq_string_invalid_task_from_raw() {
        assert!(ZMQString::from_raw(
            "SOMETHINGELSE|UNDEFINED".to_string()
        ).is_err())
    }

    #[test]
    fn create_process_task_from_zmq_string() {
        let task_zmqs = ZMQString::from_raw(
            "TASKDEF|PROCESS|ls".to_string()
        ).unwrap();
        assert!(Task::from_zmq_str(task_zmqs).is_ok());
    }

    #[test]
    fn create_process_task_from_zmq_string_with_args() {
        let task_zmqs = ZMQString::from_raw(
            "TASKDEF|PROCESS|ls|thisdir".to_string()
        ).unwrap();
        assert!(Task::from_zmq_str(task_zmqs).is_ok());
    }

    #[test]
    fn create_process_task_from_zmq_string_missing_command() {
        let task_zmqs = ZMQString::from_raw(
            "TASKDEF|PROCESS".to_string()
        ).unwrap();
        assert!(Task::from_zmq_str(task_zmqs).is_err());
    }

}