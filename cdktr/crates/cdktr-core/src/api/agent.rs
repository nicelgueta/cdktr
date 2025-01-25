use zeromq::ZmqMessage;

use crate::{
    models::{Task, ZMQArgs},
    server::models::RepReqError,
};

use super::traits::{APIMeta, API};

pub enum AgentAPI {
    /// Check the server is online
    Ping,

    /// Action to run a specific task. This is the main hook used by the
    /// principal to send tasks for execution to the agents
    Run(Task),
}
impl From<AgentAPI> for String {
    fn from(value: AgentAPI) -> Self {
        value.to_string()
    }
}

impl API for AgentAPI {
    fn get_meta(&self) -> Vec<APIMeta> {
        const META: [(&'static str, &'static str); 2] = [
            ("PING", "Check the server is online"),
            (
                "RUN",
                "Action to run a specific task. This is the main hook used by the principal to send tasks for execution to the agents",
            ),
        ];
        META.iter()
            .map(|(action, desc)| APIMeta::new(action.to_string(), desc.to_string()))
            .collect()
    }
    fn to_string(&self) -> String {
        match self {
            Self::Ping => "PING".to_string(),
            Self::Run(task) => {
                format!("RUN|{}", task.to_string())
            }
        }
    }
}

impl TryFrom<ZmqMessage> for AgentAPI {
    type Error = RepReqError;
    fn try_from(value: ZmqMessage) -> Result<Self, Self::Error> {
        let zmq_args: ZMQArgs = value.into();
        Self::try_from(zmq_args)
    }
}

impl TryFrom<ZMQArgs> for AgentAPI {
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
            "RUN" => Ok(Self::Run(helpers::create_task_run_payload(args)?)),
            _ => Err(RepReqError::ParseError(format!(
                "Unrecognised message type: {}",
                msg_type
            ))),
        }
    }
}
impl TryFrom<String> for AgentAPI {
    type Error = RepReqError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        let zmq_args: ZMQArgs = s.into();
        Self::try_from(zmq_args)
    }
}
impl Into<ZmqMessage> for AgentAPI {
    fn into(self) -> ZmqMessage {
        ZmqMessage::from(self.to_string())
    }
}

mod helpers {
    use crate::{
        models::{Task, ZMQArgs},
        server::models::RepReqError,
    };

    pub fn create_task_run_payload(args: ZMQArgs) -> Result<Task, RepReqError> {
        let task_res = Task::try_from(args);
        match task_res {
            Ok(task) => Ok(task),
            Err(e) => Err(RepReqError::ParseError(format!(
                "Invalid TASKDEF: {}",
                e.to_string()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AgentAPI;
    use zeromq::ZmqMessage;

    #[test]
    fn test_agent_request_from_zmq_str_all_happy() {
        const ALL_HAPPIES: [&str; 2] = ["PING", "RUN|PROCESS|echo|hello"];
        for zmq_s in ALL_HAPPIES {
            let res = AgentAPI::try_from(ZmqMessage::from(zmq_s));
            assert!(res.is_ok())
        }
    }

    #[test]
    fn test_agent_req_from_zmq_str() {
        let req_types = ["PING"];
        for rt in req_types {
            AgentAPI::try_from(ZmqMessage::from(rt))
                .expect(&format!("Failed to create AgentAPI from {}", rt));
        }
    }

    #[test]
    fn test_agent_req_from_zmq_str_invalid() {
        let rt = "IOASNDONTOTALLYFAKEASDKOADOAD";
        assert!(AgentAPI::try_from(ZmqMessage::from(rt)).is_err());
    }

    #[test]
    fn test_principal_req_from_zmq_str_invalid() {
        let rt = "IOASNDONTOTALLYFAKEASDKOADOAD";
        assert!(AgentAPI::try_from(ZmqMessage::from(rt)).is_err());
    }
}
