/// Custom logger that captures logs to a memory buffer instead of stdout
/// This prevents logs from corrupting the TUI display
use log::{Level, Metadata, Record, SetLoggerError};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};

const MAX_LOG_LINES: usize = 50_000;

/// A log entry with timestamp and formatted message
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

impl LogEntry {
    pub fn format(&self) -> String {
        format!(
            "[{}] {} {}: {}",
            self.timestamp, self.level, self.target, self.message
        )
    }
}

/// Thread-safe log buffer
#[derive(Clone)]
pub struct LogBuffer {
    logs: Arc<RwLock<VecDeque<LogEntry>>>,
}

impl LogBuffer {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_LOG_LINES))),
        }
    }

    pub fn add_log(&self, entry: LogEntry) {
        let mut logs = self.logs.write().unwrap();

        // Remove oldest log if we've hit the limit
        if logs.len() >= MAX_LOG_LINES {
            logs.pop_front();
        }

        logs.push_back(entry);
    }

    pub fn get_logs(&self) -> Vec<String> {
        self.logs
            .read()
            .unwrap()
            .iter()
            .map(|entry| entry.format())
            .collect()
    }

    pub fn get_recent_logs(&self, count: usize) -> Vec<String> {
        let logs = self.logs.read().unwrap();
        let start = logs.len().saturating_sub(count);
        logs.iter()
            .skip(start)
            .map(|entry| entry.format())
            .collect()
    }
}

/// Custom logger that writes to memory buffer
pub struct BufferedLogger {
    buffer: LogBuffer,
}

impl BufferedLogger {
    pub fn new(buffer: LogBuffer) -> Self {
        Self { buffer }
    }
}

impl log::Log for BufferedLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let entry = LogEntry {
                timestamp: chrono::Local::now()
                    .format("%Y-%m-%d %H:%M:%S%.3f")
                    .to_string(),
                level: record.level().to_string(),
                target: record.target().to_string(),
                message: format!("{}", record.args()),
            };

            self.buffer.add_log(entry);
        }
    }

    fn flush(&self) {}
}

/// Initialize the buffered logger and return the buffer for reading logs
pub fn init_memory_logger() -> Result<LogBuffer, SetLoggerError> {
    let buffer = LogBuffer::new();
    let logger = BufferedLogger::new(buffer.clone());

    // Set the logger - if it fails, the buffer will still work but won't capture logs
    // We don't print to stderr here because that would corrupt the TUI
    let _ = log::set_boxed_logger(Box::new(logger));
    log::set_max_level(log::LevelFilter::Debug);

    Ok(buffer)
}

/// Initialize the buffered logger with a provided buffer
pub fn init_buffered_logger(buffer: LogBuffer) -> Result<(), SetLoggerError> {
    let logger = BufferedLogger::new(buffer);
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(log::LevelFilter::Debug);
    Ok(())
}
