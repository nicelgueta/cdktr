mod task;

pub use task::Task;

mod exceptions {
    #[derive(Debug, PartialEq)]
    pub enum ZMQParseError {
        ParseError(String),
        InvalidMessageType,
        InvalidTaskType
    }
    impl ZMQParseError {
        pub fn to_string(&self) -> String {
            match self {
                Self::ParseError(msg) => format!("ParseError: {msg}"),
                Self::InvalidMessageType => String::from("Invalid message type"),
                Self::InvalidTaskType => String::from("Invalid task type")
            }
        }
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
    /// Standard task definition for a task without a specific executor context
    /// eg.
    /// TASKDEF|PROCESS|ls|thisdir
    TaskDef,
}
impl ZMQMessageType {
    pub fn new(token: &str) -> Result<Self, exceptions::ZMQParseError> {
        match token {
            "TASKDEF" => Ok(Self::TaskDef),
            _ => Err(exceptions::ZMQParseError::InvalidMessageType)
        }
    }
    // TODO:
    // pub fn as_str(&self) -> &'static str {
    //     match self {
    //         Self::TaskDef => "TASKDEF",
    //         Self::ExeTaskDef => "EXETASKDEF"
    //     }
    // }
}


pub mod traits {
    use tokio::sync::mpsc::Sender;
    use async_trait::async_trait;
    use crate::utils::AsyncQueue;

    use super::FlowExecutionResult;

    /// An Executor is a trait that defines the interface for components that
    /// are responsible for executing tasks. The executor is responsible for
    /// running the task and sending the result back to the caller
    #[async_trait]
    pub trait Executor {
        fn new(command: &str, args: Option<Vec<String>>) -> Self ;
        async fn run(&self, tx: Sender<String>) -> FlowExecutionResult;
    }

    /// The event listener trait is for implementing components that 
    /// listen to external events and place onto a Queue. T refers to 
    /// the item that will be placed on the queue upon each event.
    #[async_trait]
    pub trait EventListener<T> {
        async fn start_listening_loop(&self, out_queue: AsyncQueue<T>) ;

    }

}


#[cfg(test)]
mod tests {
    use zeromq::ZmqMessage;
    use std::collections::VecDeque;
    use super::*;

    #[test]
    fn test_task_from_zmq_vec(){
        let zmq_v: Vec<String> = vec!["PROCESS","ls","thisdir"].iter().map(|x|x.to_string()).collect();
        let zmq_vd = VecDeque::from(zmq_v);
        assert!(Task::try_from(zmq_vd).is_ok());
    }

    #[test]
    fn zmq_message_type_taskdef() {
        assert!(ZMQMessageType::new("TASKDEF").is_ok());
    }

    #[test]
    fn zmq_message_type_invalid() {
        assert!(ZMQMessageType::new("invalidinvalid").is_err());
    }

    // TASKDEF
    #[test]
    fn create_invalid_taskdef_from_zmq_message() {
        let msg = ZmqMessage::from("TASKDEF|WHATISTHIS?|ls");
        assert!(Task::try_from(msg).is_err());
    }

    #[test]
    fn create_process_taskdef_from_zmq_message() {
        let msg = ZmqMessage::from("TASKDEF|PROCESS|ls");
        assert!(Task::try_from(msg).is_ok());
    }

    #[test]
    fn create_process_taskdef_from_zmq_msg_with_args() {
        let msg = ZmqMessage::from("TASKDEF|PROCESS|ls|thisdir");
        assert!(Task::try_from(msg).is_ok());
    }

    #[test]
    fn create_process_taskdef_from_zmq_string_missing_command() {
        let msg = ZmqMessage::from("TASKDEF|PROCESS");
        assert!(Task::try_from(msg).is_err());
    }


}