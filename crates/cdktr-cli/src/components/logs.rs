use std::time::SystemTime;

/// Log management CLI
/// This allows you to tail logs from the principal log manager
/// and filter them by workflow ID
#[derive(clap::Args)]
#[command(version, about, long_about = None)]
pub struct LogArgs {
    /// The log level to set for the application
    #[arg(long, short, default_value = "info")]
    pub log_level: String,

    /// Tail the log stream instead of reading
    /// stored logs
    #[arg(long, short)]
    pub tail: bool,

    /// The workflow ID to filter logs by
    /// if not provided, all logs will be shown
    /// that are received by the principal log manager
    #[arg(long, short)]
    pub workflow_id: Option<String>,

    /// Filter logs by a specific workflow instance
    /// id
    #[arg(long, short)]
    pub workflow_instance_id: Option<String>,

    /// The number of log lines to return. Returns all
    /// if not provided
    #[arg(long, short)]
    pub number: Option<usize>,

    /// Lower bound tiemstamp for which logs should be read. Inclusive.
    #[arg(long, short, value_parser = humantime::parse_rfc3339_weak)]
    pub start_datetime_utc: Option<SystemTime>,

    /// Upper bound timestamp for which logs
    /// should be retrieved. Non-inclusive.
    #[arg(long, short, value_parser = humantime::parse_rfc3339_weak)]
    pub end_datetime_utc: Option<SystemTime>,
}
