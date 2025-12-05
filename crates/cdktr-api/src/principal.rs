use super::traits::{API, APIMeta};
use zeromq::ZmqMessage;

use cdktr_core::{
    exceptions::GenericError,
    models::{RunStatus, ZMQArgs},
    utils::get_principal_uri,
};

#[derive(Debug, Clone)]
pub enum PrincipalAPI {
    /// Check server is online
    Ping,
    /// Lists all workflows defined in the workflow directory
    ListWorkflowStore,
    /// Runs a task by id. Principal then adds the task to the primary
    /// work queue to be picked up by a agent worker.
    /// Args:
    ///     task_id: String
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
    ///     agent_id, workflow_id, workflow_instance_id, status
    WorkflowStatusUpdate(String, String, String, RunStatus),
    /// Allows an agent to update the principal with the status of a specific
    /// task
    /// Args:
    ///     agent_id, task_id, task_execution_id, workflow_instance_id, status
    TaskStatusUpdate(String, String, String, String, RunStatus),
    /// An endpoint that can be polled for work by Agents. Agents provide their
    /// instance id token (agent_id) and if there is work available on the task queue
    /// then the principal will pop a task from the global queue and provide it to the agent
    /// if not, it will just send a simple Success (OK) message
    /// Args:
    ///     agent_id
    FetchWorkflow(String),
    /// Run a query to read logs from the database
    /// Args:
    ///     end_timestamp_ms (optional): filter to results older than this timestamp.
    ///         Defaults to current time of not specified.
    ///     start_timestamp_ms (optional): filter results greater or equal to this timestamp.
    ///         Defaults to end_timestamp - 24h if not set.
    ///     workflow_id (optional): filter results by the id of the workflow. Returns all
    ///         if not set.
    ///     workflow_instance_id (optional): filter results by a specific workflow instance.
    ///         returns any if not set.
    ///     verbose: Full instance names in logs
    QueryLogs(
        Option<u64>,
        Option<u64>,
        Option<String>,
        Option<String>,
        bool,
    ),
    /// Get recent workflow status updates (last 10 workflows)
    GetRecentWorkflowStatuses,
    /// Get list of all registered agents with their metadata
    GetRegisteredAgents,
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
                                let status = RunStatus::try_from(status)?;
                                Ok(Self::WorkflowStatusUpdate(
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
                            Some(workflow_instance_id) => match args.next() {
                                Some(status) => {
                                    let status = RunStatus::try_from(status)?;
                                    Ok(Self::TaskStatusUpdate(
                                        agent_id,
                                        task_id,
                                        task_exe_id,
                                        workflow_instance_id,
                                        status,
                                    ))
                                }
                                None => Err(GenericError::ParseError(
                                    "Missing arg TASK_STATUS".to_string(),
                                )),
                            },
                            None => Err(GenericError::ParseError(
                                "Missing arg WORKFLOW_INSTANCE_ID".to_string(),
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
            "QUERYLOGS" => match args.next() {
                Some(end_ts) => {
                    let end_ts_opt = if end_ts.len() > 0 {
                        Some(end_ts.parse().map_err(|_e| {
                            GenericError::ParseError("Not a valid end timestamp".to_string())
                        })?)
                    } else {
                        None
                    };
                    match args.next() {
                        Some(start_ts) => {
                            let start_ts_opt = if start_ts.len() > 0 {
                                Some(start_ts.parse().map_err(|_e| {
                                    GenericError::ParseError(
                                        "Not a valid end timestamp".to_string(),
                                    )
                                })?)
                            } else {
                                None
                            };
                            match args.next() {
                                Some(wf_id) => {
                                    let wf_id_opt =
                                        if wf_id.len() > 0 { Some(wf_id) } else { None };
                                    match args.next() {
                                        Some(wf_ins_id) => {
                                            let wf_ins_id_opt = if wf_ins_id.len() > 0 {
                                                Some(wf_ins_id)
                                            } else {
                                                None
                                            };
                                            Ok(Self::QueryLogs(
                                                end_ts_opt,
                                                start_ts_opt,
                                                wf_id_opt,
                                                wf_ins_id_opt,
                                                match args.next() {
                                                    Some(v) => v.len() > 0,
                                                    None => false,
                                                },
                                            ))
                                        }
                                        None => Err(GenericError::ParseError(
                                            "Missing WORKFLOW_INSTANCE_ID parameter".to_string(),
                                        )),
                                    }
                                }
                                None => Err(GenericError::ParseError(
                                    "Missing WORKFLOW_ID parameter".to_string(),
                                )),
                            }
                        }
                        None => Err(GenericError::ParseError(
                            "Missing START_TIMESTAMP parameter".to_string(),
                        )),
                    }
                }
                None => Err(GenericError::ParseError(
                    "Missing END_TIMESTAMP".to_string(),
                )),
            },
            "GETRECENTSTATUSES" => Ok(Self::GetRecentWorkflowStatuses),
            "GETREGISTEREDAGENTS" => Ok(Self::GetRegisteredAgents),
            _ => Err(GenericError::ParseError(format!(
                "Unrecognised message type: {}",
                msg_type
            ))),
        }
    }
}

impl API for PrincipalAPI {
    fn get_tcp_uri(&self) -> String {
        get_principal_uri()
    }
    fn get_meta(&self) -> Vec<APIMeta> {
        const META: [(&'static str, &'static str); 9] = [
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
                "Allows an agent to fetch a unit of work from the principal task queue. Returns a success message if there is no work to do.",
            ),
            ("QUERYLOGS", "Queries logs from the main principal database"),
            (
                "GETLATESTSTATUS",
                "Get latest status update for a workflow (workflow_id)",
            ),
            (
                "GETREGISTEREDAGENTS",
                "Get list of all registered agents with their metadata",
            ),
        ];
        META.iter()
            .map(|(action, desc)| APIMeta::new(action.to_string(), desc.to_string()))
            .collect()
    }
    fn to_string(&self) -> String {
        match self {
            Self::Ping => "PING".to_string(),
            Self::RunTask(task_id) => format!("RUNTASK\x01{task_id}"),
            Self::ListWorkflowStore => "LSWORKFLOWS".to_string(),
            Self::RegisterAgent(agent_id) => {
                format!("REGISTERAGENT\x01{agent_id}")
            }
            Self::WorkflowStatusUpdate(agent_id, task_id, task_exe_id, status) => {
                let status = status.to_string();
                format!(
                    "AGENTWORKFLOWSTATUS\x01{agent_id}\x01{task_id}\x01{task_exe_id}\x01{status}"
                )
            }
            Self::TaskStatusUpdate(
                agent_id,
                task_id,
                task_exe_id,
                workflow_instance_id,
                status,
            ) => {
                let status = status.to_string();
                format!(
                    "AGENTTASKSTATUS\x01{agent_id}\x01{task_id}\x01{task_exe_id}\x01{workflow_instance_id}\x01{status}"
                )
            }
            Self::FetchWorkflow(agent_id) => {
                format!("FETCHWORKFLOW\x01{agent_id}")
            }
            Self::QueryLogs(end_ts, start_ts, wf_id, wf_ins_id, verbose) => {
                format!(
                    "QUERYLOGS\x01{}\x01{}\x01{}\x01{}\x01{}",
                    if let Some(ts) = end_ts {
                        ts.to_string()
                    } else {
                        "".to_string()
                    },
                    if let Some(ts) = start_ts {
                        ts.to_string()
                    } else {
                        "".to_string()
                    },
                    wf_id.clone().unwrap_or("".to_string()),
                    wf_ins_id.clone().unwrap_or("".to_string()),
                    if *verbose { "v" } else { "" }
                )
            }
            Self::GetRecentWorkflowStatuses => "GETRECENTSTATUSES".to_string(),
            Self::GetRegisteredAgents => "GETREGISTEREDAGENTS".to_string(),
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
        let req_types = ["PING", "FETCHWORKFLOW\x011234"];
        for rt in req_types {
            PrincipalAPI::try_from(ZmqMessage::from(rt))
                .expect(&format!("Failed to create AgentAPI from {}", rt));
        }
    }
}
