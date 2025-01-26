use super::{exceptions::ZMQParseError, ZMQArgs};
use crate::executors::ProcessTask;
use serde::Deserialize;

/// A Task is the encapsulation provided for single unit of work defined and utilised
/// by difference components of the system. On the ZMQ sockets, it's encoded as a
/// pipe-delimited string with the first token being `TASKDEF` and the second being the
/// uppercase representation of a TaskType enum to determine the type.
/// A Task type defines the types of tasks supported by cdktr for execution.
/// The value of each enum must define the struct configuration for each task
#[derive(Debug, PartialEq, Clone, Deserialize)]
pub enum Task {
    Process(ProcessTask),
}

impl Task {
    pub fn to_string(&self) -> String {
        match self {
            Self::Process(pt) => {
                let mut tokens = vec!["PROCESS".to_string()];
                tokens.push(pt.command.clone());
                if let Some(args) = &pt.args {
                    for arg in args {
                        tokens.push(arg.clone())
                    }
                };
                tokens.join("|")
            }
        }
    }
}
impl TryFrom<ZMQArgs> for Task {
    type Error = ZMQParseError;
    fn try_from(mut zmq_args: ZMQArgs) -> Result<Self, Self::Error> {
        let typ_tok = if let Some(token) = zmq_args.next() {
            token
        } else {
            return Err(ZMQParseError::ParseError(
                "Missing token to denote task type".to_string(),
            ));
        };
        match typ_tok.as_str() {
            "PROCESS" => {
                let command = if let Some(arg) = zmq_args.next() {
                    arg
                } else {
                    return Err(ZMQParseError::ParseError(
                        "Missing tokens for PROCESS msg. Expected tokens COMMAND and (optional) ARGS"
                            .to_string(),
                    ));
                };
                let args = if zmq_args.len() < 1 {
                    None
                } else {
                    Some(zmq_args.into())
                };
                Ok(Task::Process(ProcessTask { command, args }))
            }
            _ => Err(ZMQParseError::InvalidTaskType),
        }
    }
}

impl TryFrom<String> for Task {
    type Error = ZMQParseError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Task::try_from(ZMQArgs::from(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_from_zmq_vec() {
        let zmq_args = ZMQArgs::from(
            vec!["PROCESS", "ls", "thisdir"]
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>(),
        );
        assert!(Task::try_from(zmq_args).is_ok());
    }

    #[test]
    fn test_task_from_zmq_vec_invalid_task_type() {
        let zmq_args = ZMQArgs::from(
            vec!["FAKEWHAT", "ls", "thisdir"]
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>(),
        );
        assert!(Task::try_from(zmq_args).is_err());
    }

    // PROCESS
    #[test]
    fn test_process_task_from_args_no_extra_args() {
        let zmq_args = ZMQArgs::from(
            vec!["PROCESS", "ls"]
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>(),
        );
        assert!(Task::try_from(zmq_args).is_ok());
    }

    #[test]
    fn test_process_task_from_args_missing_command() {
        let zmq_args = ZMQArgs::from(
            vec!["PROCESS"]
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>(),
        );
        assert!(Task::try_from(zmq_args).is_err());
    }

    #[test]
    fn test_task_from_string() {
        let st = String::from("PROCESS|ls");
        assert!(Task::try_from(st).is_ok())
    }
}
