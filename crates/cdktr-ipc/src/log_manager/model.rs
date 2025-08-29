use std::sync::Arc;

use cdktr_core::{
    exceptions::{GenericError, ZMQParseError, cdktr_result},
    models::ZMQArgs,
    zmq_helpers::format_zmq_msg_str,
};
use cdktr_db::DBRecordBatch;
use duckdb::{
    arrow::{
        array::{RecordBatch, StringArray, StringBuilder, UInt64Array, UInt64Builder},
        datatypes::{DataType, Field, Schema},
    },
    params,
};
use zeromq::ZmqMessage;

#[derive(Clone, PartialEq, Debug)]
pub struct LogMessage {
    pub workflow_id: String,
    pub workflow_name: String,
    pub workflow_instance_id: String,
    pub task_name: String,
    pub task_instance_id: String,
    pub timestamp_ms: u64,
    pub level: String,
    pub payload: String,
}

impl LogMessage {
    pub fn new(
        workflow_id: String,
        workflow_name: String,
        workflow_instance_id: String,
        task_name: String,
        task_instance_id: String,
        timestamp_ms: u64,
        level: String,
        payload: String,
    ) -> Self {
        LogMessage {
            workflow_id,
            workflow_name,
            workflow_instance_id,
            task_name,
            task_instance_id,
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
            "[{} {}] [{}/{}] {}",
            timestring, self.level, self.workflow_name, self.task_name, self.payload
        )
    }
    /// format the message including the workflow id
    pub fn format_full(&self) -> String {
        let timestring = chrono::DateTime::from_timestamp_millis(self.timestamp_ms as i64)
            .unwrap()
            .to_rfc3339();
        format!(
            "[{} {}] [{}={} / {}={}] {}",
            timestring,
            self.level,
            self.workflow_name,
            self.workflow_instance_id,
            self.task_name,
            self.task_instance_id,
            self.payload
        )
    }
}

impl DBRecordBatch<LogMessage> for Vec<LogMessage> {
    fn from_record_batch(batch: RecordBatch) -> Result<Vec<LogMessage>, GenericError> {
        let workflow_id = batch
            .column(batch.schema().index_of("workflow_id").unwrap())
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();

        let workflow_name = batch
            .column(batch.schema().index_of("workflow_name").unwrap())
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();

        let workflow_instance_id = batch
            .column(batch.schema().index_of("workflow_instance_id").unwrap())
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();

        let task_name = batch
            .column(batch.schema().index_of("task_name").unwrap())
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();

        let task_instance_id = batch
            .column(batch.schema().index_of("task_instance_id").unwrap())
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();

        let timestamp_ms = batch
            .column(batch.schema().index_of("timestamp_ms").unwrap())
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap();

        let level = batch
            .column(batch.schema().index_of("level").unwrap())
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();

        let payload = batch
            .column(batch.schema().index_of("payload").unwrap())
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();

        // Now build Vec<LogMessage>
        Ok((0..batch.num_rows())
            .map(|i| LogMessage {
                workflow_id: workflow_id.value(i).to_string(),
                workflow_name: workflow_name.value(i).to_string(),
                workflow_instance_id: workflow_instance_id.value(i).to_string(),
                task_name: task_name.value(i).to_string(),
                task_instance_id: task_instance_id.value(i).to_string(),
                timestamp_ms: timestamp_ms.value(i),
                level: level.value(i).to_string(),
                payload: payload.value(i).to_string(),
            })
            .collect())
    }
    fn to_record_batch(&self) -> Result<RecordBatch, GenericError> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("workflow_id", DataType::Utf8, false),
            Field::new("workflow_name", DataType::Utf8, false),
            Field::new("workflow_instance_id", DataType::Utf8, false),
            Field::new("task_name", DataType::Utf8, false),
            Field::new("task_instance_id", DataType::Utf8, false),
            Field::new("timestamp_ms", DataType::UInt64, false),
            Field::new("level", DataType::Utf8, false),
            Field::new("payload", DataType::Utf8, false),
        ]));
        let mut workflow_id = StringBuilder::new();
        let mut workflow_name = StringBuilder::new();
        let mut workflow_instance_id = StringBuilder::new();
        let mut task_name = StringBuilder::new();
        let mut task_instance_id = StringBuilder::new();
        let mut timestamp_ms = UInt64Builder::new();
        let mut level = StringBuilder::new();
        let mut payload = StringBuilder::new();

        for log in self {
            workflow_id.append_value(&log.workflow_id);
            workflow_name.append_value(&log.workflow_name);
            workflow_instance_id.append_value(&log.workflow_instance_id);
            task_name.append_value(&log.task_name);
            task_instance_id.append_value(&log.task_instance_id);
            timestamp_ms.append_value(log.timestamp_ms);
            level.append_value(&log.level);
            payload.append_value(&log.payload);
        }

        let arrays = vec![
            Arc::new(workflow_id.finish()) as _,
            Arc::new(workflow_name.finish()) as _,
            Arc::new(workflow_instance_id.finish()) as _,
            Arc::new(task_name.finish()) as _,
            Arc::new(task_instance_id.finish()) as _,
            Arc::new(timestamp_ms.finish()) as _,
            Arc::new(level.finish()) as _,
            Arc::new(payload.finish()) as _,
        ];

        Ok(RecordBatch::try_new(schema, arrays).map_err(|e| {
            GenericError::DBError(format!(
                "Failed to create arrow record batch for db insertion. Orig error: {}",
                e.to_string()
            ))
        })?)
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
            workflow_id: zmq_args.next().unwrap(),
            workflow_name: zmq_args.next().unwrap(),
            workflow_instance_id: zmq_args.next().unwrap(),
            task_name: zmq_args.next().unwrap(),
            task_instance_id: zmq_args.next().unwrap(),
            timestamp_ms: cdktr_result(zmq_args.next().unwrap().parse())?,
            level: zmq_args.next().unwrap(),
            payload: zmq_args.next().unwrap(),
        })
    }
}

impl Into<ZmqMessage> for LogMessage {
    fn into(self) -> ZmqMessage {
        ZmqMessage::from(format_zmq_msg_str(vec![
            &self.workflow_id,
            &self.workflow_name,
            &self.workflow_instance_id,
            &self.task_name,
            &self.task_instance_id,
            &self.timestamp_ms.to_string(),
            &self.level,
            &self.payload,
        ]))
    }
}
