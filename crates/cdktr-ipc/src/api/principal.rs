use super::traits::{APIMeta, API};
use zeromq::ZmqMessage;

use cdktr_core::{
    exceptions::GenericError,
    models::{TaskStatus, ZMQArgs},
};

// TODO: make an extension of AgentAPI
#[derive(Debug)]
pub enum PrincipalAPI {
    /// Check server is online
    Ping,
    /// Lists all workflows defined in the workflow directory
    ListWorkflowStore,
    /// Runs a task by id. Principal then adds the task to the primary
    /// work queue to be picked up by a agent worker.
    /// Args:
    ///     task_id: i64
    RunTask(String),
    /// Allows an agent to register itself with the principal
    /// can register its presence. If the agent
    /// is already registered then this behaves in a similar way to
    /// a PING/PONG
    /// Args:
    ///     agent_id
    RegisterAgent(String),
    /// Allows an agent to update the principal with the status of a specific
    /// workflow
    /// Args:
    ///     agent_id, task_id, task_execution_id, status
    AgentWorkflowStatusUpdate(String, String, String, TaskStatus),
    /// Allows an agent to update the principal with the status of a specific
    /// task
    /// Args:
    ///     agent_id, task_id, task_execution_id, status
    AgentTaskStatusUpdate(String, String, String, TaskStatus),
    /// An endpoint that can be polled for work by Agents. Agents provide their
    /// instance id token (agent_id) and if there is work available on the task queue
    /// then the principal will pop a task from the global queue and provide it to the agent
    /// if not, it will just send a simple Success (OK) message
    /// Args:
    ///     agent_id
    FetchWorkflow(String),
}

impl TryFrom<ZMQArgs> for PrincipalAPI {
    type Error = GenericError;
    fn try_from(mut args: ZMQArgs) -> Result<Self, Self::Error> {
        let msg_type = if let Some(token) = args.next() {
            token
        } else {
            return Err(GenericError::ParseError(format!("Empty message")));
        };
        match msg_type.as_str() {
            "PING" => Ok(Self::Ping),
            "LSWORKFLOWS" => Ok(Self::ListWorkflowStore),
            "RUNTASK" => Ok(Self::RunTask(helpers::create_run_task_payload(args)?)),
            "REGISTERAGENT" => match args.next() {
                Some(agent_id) => Ok(Self::RegisterAgent(agent_id)),
                None => Err(GenericError::ParseError("Missing arg AGENT_ID".to_string())),
            },
            "AGENTWORKFLOWSTATUS" => match args.next() {
                Some(agent_id) => match args.next() {
                    Some(task_id) => match args.next() {
                        Some(task_exe_id) => match args.next() {
                            Some(status) => {
                                let status = TaskStatus::try_from(status)?;
                                Ok(Self::AgentWorkflowStatusUpdate(
                                    agent_id,
                                    task_id,
                                    task_exe_id,
                                    status,
                                ))
                            }
                            None => Err(GenericError::ParseError(
                                "Missing arg TASK_STATUS".to_string(),
                            )),
                        },
                        None => Err(GenericError::ParseError(
                            "Missing arg TASK_EXECUTION_ID".to_string(),
                        )),
                    },
                    None => Err(GenericError::ParseError("Missing arg TASK_ID".to_string())),
                },
                None => Err(GenericError::ParseError("Missing arg AGENT_ID".to_string())),
            },
            "AGENTTASKSTATUS" => match args.next() {
                Some(agent_id) => match args.next() {
                    Some(task_id) => match args.next() {
                        Some(task_exe_id) => match args.next() {
                            Some(status) => {
                                let status = TaskStatus::try_from(status)?;
                                Ok(Self::AgentTaskStatusUpdate(
                                    agent_id,
                                    task_id,
                                    task_exe_id,
                                    status,
                                ))
                            }
                            None => Err(GenericError::ParseError(
                                "Missing arg TASK_STATUS".to_string(),
                            )),
                        },
                        None => Err(GenericError::ParseError(
                            "Missing arg TASK_EXECUTION_ID".to_string(),
                        )),
                    },
                    None => Err(GenericError::ParseError("Missing arg TASK_ID".to_string())),
                },
                None => Err(GenericError::ParseError("Missing arg AGENT_ID".to_string())),
            },
            "FETCHWORKFLOW" => match args.next() {
                Some(agent_id) => Ok(Self::FetchWorkflow(agent_id)),
                None => Err(GenericError::ParseError("Missing agent id".to_string())),
            },
            _ => Err(GenericError::ParseError(format!(
                "Unrecognised message type: {}",
                msg_type
            ))),
        }
    }
}

impl API for PrincipalAPI {
    fn get_meta(&self) -> Vec<APIMeta> {
        const META: [(&'static str, &'static str); 6] = [
            ("PING", "Check server is online"),
            (
                "LSWORKFLOWS",
                "Lists all workflows defined in the workflow directory",
            ),
            (
                "REGISTERAGENT",
                "Allows an agent to register itself with the principal",
            ),
            (
                "AGENTWORKFLOWSTATUS",
                "Allows an agent to update the principal with the status of a specific workflow",
            ),
            (
                "AGENTTASKSTATUS",
                "Allows an agent to update the principal with the status of a specific task",
            ),
            (
                "FETCHWORKFLOW",
                "Allows an agent to fetch a unit of work from the principal task queue. Returns a success message if there is no work to do."
            )
        ];
        META.iter()
            .map(|(action, desc)| APIMeta::new(action.to_string(), desc.to_string()))
            .collect()
    }
    fn to_string(&self) -> String {
        match self {
            Self::Ping => "PING".to_string(),
            Self::RunTask(task_id) => format!("RUNTASK|{task_id}"),
            Self::ListWorkflowStore => "LSWORKFLOWS".to_string(),
            Self::RegisterAgent(agent_id) => {
                format!("REGISTERAGENT|{agent_id}")
            }
            Self::AgentWorkflowStatusUpdate(agent_id, task_id, task_exe_id, status) => {
                let status = status.to_string();
                format!("AGENTWORKFLOWSTATUS|{agent_id}|{task_id}|{task_exe_id}|{status}")
            }
            Self::AgentTaskStatusUpdate(agent_id, task_id, task_exe_id, status) => {
                let status = status.to_string();
                format!("AGENTTASKSTATUS|{agent_id}|{task_id}|{task_exe_id}|{status}")
            }
            Self::FetchWorkflow(agent_id) => {
                format!("FETCHWORKFLOW|{agent_id}")
            }
        }
    }
}

impl TryFrom<ZmqMessage> for PrincipalAPI {
    type Error = GenericError;
    fn try_from(zmq_msg: ZmqMessage) -> Result<Self, Self::Error> {
        let zmq_args: ZMQArgs = zmq_msg.into();
        Self::try_from(zmq_args)
    }
}
impl TryFrom<String> for PrincipalAPI {
    type Error = GenericError;
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
    use cdktr_core::{exceptions::GenericError, models::ZMQArgs};

    pub fn create_run_task_payload(mut args: ZMQArgs) -> Result<String, GenericError> {
        let task_id = if let Some(task_id) = args.next() {
            task_id
        } else {
            return Err(GenericError::ParseError(
                "Request is missing task_id".to_string(),
            ));
        };
        match task_id.parse() {
            Ok(v) => Ok(v),
            Err(e) => Err(GenericError::ParseError(format!(
                "Unable to create integer from task_id '{}'. Error: {}",
                &task_id,
                e.to_string()
            ))),
        }
    }

    #[cfg(test)]
    mod tests {}
}

#[cfg(test)]
mod tests {
    use super::PrincipalAPI;
    use zeromq::ZmqMessage;

    #[test]
    fn test_principal_req_from_zmq_str() {
        let req_types = ["PING", "FETCHWORKFLOW|1234"];
        for rt in req_types {
            PrincipalAPI::try_from(ZmqMessage::from(rt))
                .expect(&format!("Failed to create AgentAPI from {}", rt));
        }
    }
}
