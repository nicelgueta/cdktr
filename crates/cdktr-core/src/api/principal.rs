use super::traits::{APIMeta, API};
use zeromq::ZmqMessage;

use crate::{
    db::models::NewScheduledTask,
    models::{Task, TaskStatus, ZMQArgs},
    server::models::RepReqError,
};

// TODO: make an extension of AgentAPI
#[derive(Debug)]
pub enum PrincipalAPI {
    /// Check server is online
    Ping,
    /// Creates a new scheudled task in the principal database
    CreateTask(NewScheduledTask),
    /// Lists all scheduled tasks currently stored in the database
    ListTasks,
    /// Deletes a specific scheduled task in the database by its id
    /// Args:
    ///     task_id : i32
    DeleteTask(i32),
    /// Adds a task to the primary task queue held on the principal
    /// that agents reuest work from
    /// Args:
    ///     task: Task
    AddTask(Task),
    /// Allows an agent to register itself with the principal
    /// so that the principal can set a heartbeat for it. If the agent
    /// is already registered then this behaves in a similar way to
    /// a PING/PONG
    /// Args:
    ///     agent_id
    RegisterAgent(String),
    /// Allows an agent to update the principal with the status of a specific
    /// task
    /// Args:
    ///     agent_id, task_id, status
    AgentTaskStatusUpdate(String, String, TaskStatus),
    /// An endpoint that can be polled for work by Agents. Agents provide their
    /// instance id token (agent_id) and if there is work available on the task queue
    /// then the principal will pop a task from the global queue and provide it to the agent
    /// if not, it will just send a simple Success (OK) message
    /// Args:
    ///     agent_id
    FetchTask(String),
}

impl TryFrom<ZMQArgs> for PrincipalAPI {
    type Error = RepReqError;
    fn try_from(mut args: ZMQArgs) -> Result<Self, Self::Error> {
        let msg_type = if let Some(token) = args.next() {
            token
        } else {
            return Err(RepReqError::ParseError(format!("Empty message")));
        };
        match msg_type.as_str() {
            // "GET_TASKS" => Ok(Self::GetTasks),
            "PING" => Ok(Self::Ping),
            "CREATETASK" => Ok(Self::CreateTask(helpers::create_new_task_payload(args)?)),
            "LISTTASKS" => Ok(Self::ListTasks),
            "DELETETASK" => Ok(Self::DeleteTask(helpers::delete_task_payload(args)?)),
            "ADDTASK" => {
                let task = helpers::create_run_task_payload(args)?;
                Ok(Self::AddTask(task))
            }
            "REGISTERAGENT" => match args.next() {
                Some(agent_id) => Ok(Self::RegisterAgent(agent_id)),
                None => Err(RepReqError::ParseError("Missing arg AGENT_ID".to_string())),
            },
            "AGENTTASKSTATUS" => match args.next() {
                Some(agent_id) => match args.next() {
                    Some(task_id) => match args.next() {
                        Some(status) => {
                            let status = TaskStatus::try_from(status)?;
                            Ok(Self::AgentTaskStatusUpdate(agent_id, task_id, status))
                        }
                        None => Err(RepReqError::ParseError(
                            "Missing arg TASK_STATUS".to_string(),
                        )),
                    },
                    None => Err(RepReqError::ParseError("Missing arg TASK_ID".to_string())),
                },
                None => Err(RepReqError::ParseError("Missing arg AGENT_ID".to_string())),
            },
            "FETCHTASK" => match args.next() {
                Some(agent_id) => Ok(Self::FetchTask(agent_id)),
                None => Err(RepReqError::ParseError("Missing agent id".to_string())),
            },
            _ => Err(RepReqError::ParseError(format!(
                "Unrecognised message type: {}",
                msg_type
            ))),
        }
    }
}

impl API for PrincipalAPI {
    fn get_meta(&self) -> Vec<APIMeta> {
        const META: [(&'static str, &'static str); 8] = [
            ("PING", "Check server is online"),
            (
                "CREATETASK",
                "Creates a new scheudled task in the principal database",
            ),
            (
                "LISTTASKS",
                "Lists all scheduled tasks currently stored in the database",
            ),
            (
                "DELETETASK",
                "Deletes a specific scheduled task in the database by its id",
            ),
            (
                "ADDTASK",
                "Adds a task to the principal task queue for execution on an agent",
            ),
            (
                "REGISTERAGENT",
                "Allows an agent to register itself with the principal",
            ),
            (
                "AGENTTASKSTATUS",
                "Allows an agent to update the principal with the status of a specific task",
            ),
            (
                "FETCHTASK",
                "Allows an agent to fetch a unit or work from the principal task queue. Returns a success message if there is no work to do."
            )
        ];
        META.iter()
            .map(|(action, desc)| APIMeta::new(action.to_string(), desc.to_string()))
            .collect()
    }
    fn to_string(&self) -> String {
        match self {
            Self::Ping => "PING".to_string(),
            Self::CreateTask(task) => {
                let task_name = &task.task_name;
                let task_type = &task.task_type;
                let command = &task.command;
                let args = &task.args;
                let cron = &task.cron;
                let next_run_timestamp = task.next_run_timestamp;
                format!("CREATETASK|{task_name}|{task_type}|{command}|{args}|{cron}|{next_run_timestamp}")
            }
            Self::AddTask(task) => {
                let task_str: String = task.to_string();
                format!("ADDTASK|{task_str}")
            }
            Self::DeleteTask(task_id) => format!("DELETETASK|{task_id}"),
            Self::ListTasks => "LISTTASKS".to_string(),
            Self::RegisterAgent(agent_id) => {
                format!("REGISTERAGENT|{agent_id}")
            }
            Self::AgentTaskStatusUpdate(agent_id, task_id, status) => {
                let status = status.to_string();
                format!("AGENTTASKSTATUS|{agent_id}|{task_id}|{status}")
            }
            Self::FetchTask(agent_id) => {
                format!("FETCHTASK|{agent_id}")
            }
        }
    }
}

impl TryFrom<ZmqMessage> for PrincipalAPI {
    type Error = RepReqError;
    fn try_from(zmq_msg: ZmqMessage) -> Result<Self, Self::Error> {
        let zmq_args: ZMQArgs = zmq_msg.into();
        Self::try_from(zmq_args)
    }
}
impl TryFrom<String> for PrincipalAPI {
    type Error = RepReqError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        let zmq_args: ZMQArgs = s.into();
        Self::try_from(zmq_args)
    }
}
impl Into<ZmqMessage> for PrincipalAPI {
    fn into(self) -> ZmqMessage {
        ZmqMessage::from(self.to_string())
    }
}

mod helpers {
    use crate::{
        db::models::NewScheduledTask,
        models::{Task, ZMQArgs},
        server::models::RepReqError,
    };

    /// Creates a new task from the provided ZMQArgs
    /// Order for the ZMQArgs is:
    /// task_name: String - name of the task,
    /// task_type: String - type of task. eg. PROCESS,
    /// command: String - command to run,
    /// args: String - comma separated list of arguments,
    /// cron: String - cron expression for the task,
    /// next_run_timestamp: i32 - timestamp for the next run of the task - i.e. the start time.
    pub fn create_new_task_payload(mut args: ZMQArgs) -> Result<NewScheduledTask, RepReqError> {
        let task = NewScheduledTask {
            task_name: args
                .next()
                .ok_or(RepReqError::ParseError("task_name is missing".to_string()))?,
            task_type: args
                .next()
                .ok_or(RepReqError::ParseError("task_type is missing".to_string()))?,
            command: args
                .next()
                .ok_or(RepReqError::ParseError("command is missing".to_string()))?,
            args: args
                .next()
                .ok_or(RepReqError::ParseError("args is missing".to_string()))?,
            cron: args
                .next()
                .ok_or(RepReqError::ParseError("cron is missing".to_string()))?,
            next_run_timestamp: args
                .next()
                .ok_or(RepReqError::ParseError(
                    "next_run_timestamp is missing".to_string(),
                ))?
                .parse()
                .or(Err(RepReqError::ParseError(
                    "next_run_timestamp is not a valid integer".to_string(),
                )))?,
        };
        Ok(task)
    }

    pub fn create_run_task_payload(args: ZMQArgs) -> Result<Task, RepReqError> {
        match Task::try_from(args) {
            Ok(task) => Ok(task),
            Err(e) => Err(RepReqError::ParseError(format!(
                "Invalid task definition. Error: {}",
                e.to_string()
            ))),
        }
    }

    pub fn delete_task_payload(mut args: ZMQArgs) -> Result<i32, RepReqError> {
        if let Some(v) = args.next() {
            match v.parse() {
                Ok(v) => Ok(v),
                Err(e) => Err(RepReqError::ParseError(format!(
                    "Unable to create integer from value '{}'. Error: {}",
                    &v,
                    e.to_string()
                ))),
            }
        } else {
            Err(RepReqError::ParseError(
                "No payload found for DELETETASK command. Requires TASK_ID".to_string(),
            ))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::models::ZMQArgs;

        #[test]
        fn test_create_task_payload_happy_from_string() {
            // not inlcude original CREATETASK since that should be handled already
            let arg_s = "echo hello|PROCESS|echo|hello|0 3 * * * *|1720313744".to_string();
            assert!(create_new_task_payload(ZMQArgs::from(arg_s)).is_ok())
        }

        #[test]
        fn test_create_task_payload_happy_from_vec() {
            let arg_v: Vec<String> = vec![
                "echo hello",
                "PROCESS",
                "echo",
                "hello",
                "0 3 * * * *",
                "1720313744",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect();
            assert!(create_new_task_payload(ZMQArgs::from(arg_v)).is_ok())
        }

        #[test]
        fn test_create_task_payload_invalid_json() {
            let args = vec![r#"{"task_name": "#.to_string()];
            assert!(create_new_task_payload(ZMQArgs::from(args)).is_err())
        }

        #[test]
        fn test_create_task_payload_valid_json_but_not_task() {
            let args = vec![r#"{"task_name": "missing all other props"}"#.to_string()];
            assert!(create_new_task_payload(ZMQArgs::from(args)).is_err())
        }

        #[test]
        fn test_delete_task_happy() {
            let args = vec!["1".to_string()];
            assert!(delete_task_payload(ZMQArgs::from(args)).is_ok())
        }

        #[test]
        fn test_delete_task_invalid() {
            let args = vec!["not_an_int".to_string()];
            assert!(delete_task_payload(ZMQArgs::from(args)).is_err())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PrincipalAPI;
    use zeromq::ZmqMessage;

    #[test]
    fn test_principal_req_from_zmq_str() {
        let req_types = ["PING", "FETCHTASK|1234"];
        for rt in req_types {
            PrincipalAPI::try_from(ZmqMessage::from(rt))
                .expect(&format!("Failed to create AgentAPI from {}", rt));
        }
    }
}
