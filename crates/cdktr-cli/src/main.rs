use cdktr_core::{get_cdktr_setting, utils};
use cdktr_ipc::{
    instance::{start_agent, start_principal},
    log_manager::{client::LogsClient, model::LogMessage},
};
use cdktr_tui::tui_main;
use clap::Parser;
use dotenv::dotenv;
use log::{info, warn};
use models::InstanceType;
use std::env;
use std::path::Path;

use crate::components::logs::LogArgs;

mod api;
mod components;
mod models;

/// CDKTR Command Line Interface
/// You can manage your entire CDKTR setup using this CLI
#[derive(Parser)]
#[command(name = "cdktr")]
#[command(bin_name = "cdktr")]
enum CdktrCli {
    /// Open up the main CDKTR TUI
    Ui,

    /// Interact with a live principal instance
    /// for task management
    Task(api::TaskArgs),

    /// Start a principal or agent node
    Start(StartArgs),

    /// Log management CLI
    Logs(LogArgs),
}

#[derive(clap::Args)]
#[command(version, about, long_about = None)]
struct StartArgs {
    #[arg(long, short)]
    instance_type: models::InstanceType,

    #[arg(long, short)]
    port: Option<usize>,

    #[arg(long, short)]
    max_concurrent_workflows: Option<usize>,

    #[arg(long, short)]
    config: Option<std::path::PathBuf>,
}

fn setup() {
    let path_str = &get_cdktr_setting!(CDKTR_APP_DATA_DIRECTORY);
    let path_str = path_str.replace("$HOME", &env::var("HOME").expect(
        format!(
            "CDKTR_APP_DATA_DIRECTORY not set so attempted to create app data directory at {path_str} but cannot determine home directory from env vars."
        ).as_str()
    ));
    let app_data_dir = Path::new(&path_str);

    info!("Using application data directory: {:?}", app_data_dir);
    if let Err(e) = std::fs::create_dir_all(&app_data_dir) {
        warn!(
            "Failed to create application data directory {:?}: {}",
            app_data_dir, e
        );
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();
    setup();
    _main().await;
}

async fn _main() {
    let cli_instance = CdktrCli::parse();

    let principal_host = get_cdktr_setting!(CDKTR_PRINCIPAL_HOST);
    let principal_port: usize = get_cdktr_setting!(CDKTR_PRINCIPAL_PORT, usize);

    match cli_instance {
        CdktrCli::Start(args) => {
            let instance_type = args.instance_type;
            match instance_type {
                InstanceType::AGENT => {
                    let instance_id = format!("{}/AG", utils::get_instance_id());
                    info!("Starting AGENT instance: {}", &instance_id);
                    let max_concurrent_workflows = args
                        .max_concurrent_workflows
                        .unwrap_or(get_cdktr_setting!(CDKTR_AGENT_MAX_CONCURRENCY, usize));
                    info!("Agent max concurrency: {}", max_concurrent_workflows);
                    start_agent(
                        instance_id,
                        principal_host,
                        principal_port,
                        max_concurrent_workflows,
                    )
                    .await
                }

                InstanceType::PRINCIPAL => {
                    let instance_id = format!("{}/PRIN", utils::get_instance_id());
                    info!("Starting PRINCIPAL instance: {}", &instance_id);
                    if let Err(e) =
                        start_principal(principal_host, principal_port, instance_id).await
                    {
                        println!("{}", e.to_string())
                    }
                }
            }
        }
        CdktrCli::Ui => {
            let _ = tui_main().await;
            ()
        }
        CdktrCli::Task(args) => (),
        CdktrCli::Logs(args) => {
            // let log_level = args.log_level.to_lowercase();
            let print_func = if let Some(wf_id) = &args.workflow_id {
                |msg: LogMessage| println!("{}", msg.format())
            } else {
                |msg: LogMessage| println!("{}", msg.format_full())
            };
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
    }
}
