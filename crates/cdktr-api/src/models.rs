use cdktr_db::impl_dbrecordbatch;
use serde::{Deserialize, Serialize};
use zeromq::ZmqMessage;

use cdktr_core::models::ZMQArgs;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AgentInfo {
    pub agent_id: String,
    pub last_ping_timestamp: i64,
    pub running_tasks: usize,
}

impl AgentInfo {
    pub fn new(agent_id: String, last_ping_timestamp: i64, running_tasks: usize) -> Self {
        Self {
            agent_id,
            last_ping_timestamp,
            running_tasks,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct WorkflowStatusUpdate {
    workflow_id: String,
    workflow_instance_id: String,
    status: String,
    timestamp_ms: u64,
}
impl WorkflowStatusUpdate {
    pub fn new(
        workflow_id: String,
        workflow_instance_id: String,
        status: String,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            workflow_id,
            workflow_instance_id,
            status,
            timestamp_ms,
        }
    }

    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }

    pub fn workflow_instance_id(&self) -> &str {
        &self.workflow_instance_id
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn timestamp_ms(&self) -> u64 {
        self.timestamp_ms
    }
}
impl_dbrecordbatch!(
    WorkflowStatusUpdate, Vec<WorkflowStatusUpdate>, {
        workflow_id => Utf8,
        workflow_instance_id => Utf8,
        status => Utf8,
        timestamp_ms => UInt64,
    }
);

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TaskStatusUpdate {
    task_id: String,
    task_instance_id: String,
    status: String,
    timestamp_ms: u64,
}
impl TaskStatusUpdate {
    pub fn new(
        task_id: String,
        task_instance_id: String,
        status: String,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            task_id,
            task_instance_id,
            status,
            timestamp_ms,
        }
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    pub fn task_instance_id(&self) -> &str {
        &self.task_instance_id
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn timestamp_ms(&self) -> u64 {
        self.timestamp_ms
    }
}
impl_dbrecordbatch!(
    TaskStatusUpdate, Vec<TaskStatusUpdate>, {
        task_id => Utf8,
        task_instance_id => Utf8,
        status => Utf8,
        timestamp_ms => UInt64,
    }
);

#[derive(Debug)]
pub enum RepReqError {
    ParseError(String),
    Unprocessable(String),
    ServerError(String),
}
impl RepReqError {
    pub fn to_string(&self) -> String {
        match self {
            Self::ParseError(pl) => format!("PARSE ERROR: {}", pl),
            Self::Unprocessable(pl) => format!("UNPROCESSABLE: {}", pl),
            Self::ServerError(pl) => format!("SERVER ERROR: {}", pl),
        }
    }
}

/// A message that is returned to the client REQ socket.
#[derive(PartialEq, Debug)]
pub enum ClientResponseMessage {
    ClientError(String),
    ServerError(String),
    Unprocessable(String),
    Pong,
    Success,
    SuccessWithPayload(String),
    NetworkError(String),
}

impl ClientResponseMessage {
    pub fn to_string(&self) -> String {
        match self {
            Self::Pong => "PONG".to_string(),
            Self::Success => "OK".to_string(),
            Self::SuccessWithPayload(payload) => format!("SUCCESS\x01{payload}"),

            Self::ClientError(payload) => format!("CLIENTERROR\x01{payload}"),
            Self::ServerError(payload) => format!("SERVERERROR\x01{payload}"),
            Self::Unprocessable(payload) => format!("UNPROC\x01{payload}"),
            Self::NetworkError(payload) => format!("NETWORKERROR\x01{payload}"),
        }
    }

    /// Convenience method used to unpack a client message payload into just the string without
    /// the initial token that's used to denote the message type. If the message does not have a
    /// payload then just an empty string is returned
    pub fn payload(&self) -> String {
        match self {
            Self::Pong => "".to_string(),
            Self::Success => "".to_string(),
            Self::SuccessWithPayload(pl) => pl.clone(),

            Self::ClientError(pl) => pl.clone(),
            Self::ServerError(pl) => pl.clone(),
            Self::Unprocessable(pl) => pl.clone(),
            Self::NetworkError(pl) => pl.clone(),
        }
    }
}

impl From<ZmqMessage> for ClientResponseMessage {
    fn from(value: ZmqMessage) -> Self {
        let mut args: ZMQArgs = value.into();
        let msg_type = if let Some(v) = args.next() {
            v
        } else {
            return Self::ClientError("Cannot work with an empty message".to_string());
        };
        match msg_type.as_str() {
            "CLIENTERROR" => Self::ClientError(args.to_string()),
            "SERVERERROR" => Self::ServerError(args.to_string()),
            "UNPROC" => Self::Unprocessable(args.to_string()),
            "PONG" => Self::Pong,
            "OK" => Self::Success,
            "SUCCESS" => Self::SuccessWithPayload(args.to_string()),
            mt => Self::ClientError(format!("Unrecognised message type: {}", mt)),
        }
    }
}

impl Into<String> for ClientResponseMessage {
    fn into(self) -> String {
        self.to_string()
    }
}

impl Into<ZmqMessage> for ClientResponseMessage {
    fn into(self) -> ZmqMessage {
        let msg: String = self.into();
        ZmqMessage::from(msg)
    }
}

#[cfg(test)]
mod tests {
    use zeromq::ZmqMessage;

    use super::{AgentInfo, ClientResponseMessage};

    #[test]
    fn test_agent_info_serialization() {
        let agent = AgentInfo::new("test-agent-001".to_string(), 1234567890, 5);

        let json = serde_json::to_string(&agent).unwrap();
        let deserialized: AgentInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.agent_id, "test-agent-001");
        assert_eq!(deserialized.last_ping_timestamp, 1234567890);
        assert_eq!(deserialized.running_tasks, 5);
    }

    #[test]
    fn test_agent_info_vec_serialization() {
        let agents = vec![
            AgentInfo::new("agent-1".to_string(), 1000, 2),
            AgentInfo::new("agent-2".to_string(), 2000, 0),
        ];

        let json = serde_json::to_string(&agents).unwrap();
        let deserialized: Vec<AgentInfo> = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.len(), 2);
        assert_eq!(deserialized[0].agent_id, "agent-1");
        assert_eq!(deserialized[1].agent_id, "agent-2");
    }

    #[test]
    fn test_client_message_success_payload() {
        let zmq_m = ZmqMessage::from("SUCCESS\x01SOME random payload\x01with\x01other_args");
        let cli_msg = ClientResponseMessage::from(zmq_m);
        assert_eq!(
            cli_msg.payload(),
            "SOME random payload\x01with\x01other_args".to_string()
        )
    }

    #[test]
    fn test_client_message_success_payload_direct_match() {
        let zmq_m = ZmqMessage::from("SUCCESS\x01SOME random payload\x01with\x01other_args");
        let cli_msg = ClientResponseMessage::from(zmq_m);
        match cli_msg {
            ClientResponseMessage::SuccessWithPayload(pl) => {
                assert_eq!(pl, "SOME random payload\x01with\x01other_args".to_string())
            }
            _ => panic!("Expected only success payload for this test"),
        }
    }
}
