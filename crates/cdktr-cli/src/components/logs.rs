use cdktr_api::{PrincipalAPI, PrincipalClient, models::ClientResponseMessage};
use cdktr_ipc::log_manager::{client::LogsClient, model::LogMessage};
use log::error;
use log::info;
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

    /// Verbose workflow and task instance ids
    #[arg(long, short)]
    pub verbose: bool,

    /// The workflow ID to filter logs by
    /// if not provided, all logs will be shown
    /// that are received by the principal log manager
    #[arg(long, short('w'))]
    pub workflow_id: Option<String>,

    /// Filter logs by a specific workflow instance
    /// id
    #[arg(long, short('i'))]
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

pub async fn handle_logs(args: LogArgs) {
    let print_func = if args.verbose {
        |msg: LogMessage| println!("{}", msg.format_full())
    } else {
        |msg: LogMessage| println!("{}", msg.format())
    };

    if args.tail {
        tail_logs(args, print_func).await
    } else {
        info!("Querying logs from db");
        query_logs(args).await
    }
}

async fn tail_logs(args: LogArgs, print_func: impl Fn(LogMessage)) {
    let mut logs_client = match LogsClient::new(
        "cdktr-cli".to_string(),
        &args.workflow_id.unwrap_or("".to_string()),
    )
    .await
    {
        Ok(client) => client,
        Err(e) => {
            println!("{}", e.to_string());
            return;
        }
    };
    let (tx, mut rx) = tokio::sync::mpsc::channel::<LogMessage>(100);
    tokio::spawn(async move { logs_client.listen(tx, None).await });
    while let Some(msg) = rx.recv().await {
        print_func(msg);
    }
}

async fn query_logs(args: LogArgs) {
    // Create PrincipalClient for CLI
    let client = match PrincipalClient::new("cdktr-cli-logs".to_string()).await {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create PrincipalClient: {}", e.to_string());
            return;
        }
    };

    let api = PrincipalAPI::QueryLogs(
        match args.end_datetime_utc {
            Some(dt) => Some(
                dt.duration_since(SystemTime::UNIX_EPOCH)
                    .expect("Unable to create unix timestamp from end timestamp")
                    .as_millis() as u64,
            ),
            None => None,
        },
        match args.start_datetime_utc {
            Some(dt) => Some(
                dt.duration_since(SystemTime::UNIX_EPOCH)
                    .expect("Unable to create unix timestamp from end timestamp")
                    .as_millis() as u64,
            ),
            None => None,
        },
        args.workflow_id,
        args.workflow_instance_id,
        args.verbose,
    );
    let api_result = client.send(api).await;
    match api_result {
        Ok(msg) => match msg {
            ClientResponseMessage::SuccessWithPayload(payload) => {
                let logs: Vec<String> =
                    serde_json::from_str(&payload).expect("Unable to read logs from API response");
                for log_msg in logs {
                    println!("{}", log_msg)
                }
            }
            other => error!("Unexpected response: {}", other.to_string()),
        },
        Err(e) => {
            error!("{}", e.to_string())
        }
    }
}
