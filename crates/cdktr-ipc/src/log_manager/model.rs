use cdktr_core::{
    exceptions::{cdktr_result, GenericError, ZMQParseError},
    models::ZMQArgs,
};
use zeromq::ZmqMessage;

#[derive(Clone)]
pub struct LogMessage {
    pub workflow_name: String,
    pub workflow_instance_id: String,
    pub timestamp_ms: u128,
    pub level: String,
    pub payload: String,
}

impl LogMessage {
    pub fn new(
        workflow_name: String,
        workflow_instance_id: String,
        timestamp_ms: u128,
        level: String,
        payload: String,
    ) -> Self {
        LogMessage {
            workflow_name,
            workflow_instance_id,
            timestamp_ms,
            level,
            payload,
        }
    }
    pub fn format(&self) -> String {
        let timestring = chrono::DateTime::from_timestamp_millis(self.timestamp_ms as i64)
            .unwrap()
            .to_rfc3339();
        format!(
            "[{} {}] [{}] {}",
            timestring, self.level, self.workflow_instance_id, self.payload
        )
    }
    /// format the message including the workflow id
    pub fn format_full(&self) -> String {
        let timestring = chrono::DateTime::from_timestamp_millis(self.timestamp_ms as i64)
            .unwrap()
            .to_rfc3339();
        format!(
            "[{} {}] [{}/{}] {}",
            timestring, self.level, self.workflow_name, self.workflow_instance_id, self.payload
        )
    }
}

impl TryFrom<ZmqMessage> for LogMessage {
    type Error = GenericError;
    fn try_from(msg: ZmqMessage) -> Result<Self, Self::Error> {
        let mut zmq_args: ZMQArgs = msg.into();
        if zmq_args.len() < 4 {
            return Err(GenericError::ZMQParseError(ZMQParseError::ParseError(
                "LogMessage must have at least 4 parts: topic, timestamp, level and payload"
                    .to_string(),
            )));
        }
        Ok(LogMessage {
            workflow_name: zmq_args.next().unwrap(),
            workflow_instance_id: zmq_args.next().unwrap(),
            timestamp_ms: cdktr_result(zmq_args.next().unwrap().parse())?,
            level: zmq_args.next().unwrap(),
            payload: zmq_args.next().unwrap(),
        })
    }
}

impl Into<ZmqMessage> for LogMessage {
    fn into(self) -> ZmqMessage {
        ZmqMessage::from(format!(
            "{}|{}|{}|{}|{}",
            self.workflow_name,
            self.workflow_instance_id,
            self.timestamp_ms,
            self.level,
            self.payload
        ))
    }
}
