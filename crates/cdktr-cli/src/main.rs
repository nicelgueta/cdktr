use clap::Parser;
use dotenv::dotenv;
use log::{error, info, warn};
use models::InstanceType;
use std::env;

use cdktr_core::{get_cdktr_setting, utils};
use cdktr_ipc::instance::{start_agent, start_principal};
use cdktr_tui::tui_main;

mod api;
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

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();
    _main().await;
}

async fn _main() {
    let cli_instance = CdktrCli::parse();

    let principal_host = get_cdktr_setting!(CDKTR_PRINCIPAL_HOST);
    let principal_port: usize = get_cdktr_setting!(CDKTR_PRINCIPAL_PORT)
        .parse()
        .expect("CDKTR_PRINCIPAL_PORT must be a valid number");

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
                    start_principal(principal_host, principal_port, instance_id).await
                }
            }
        }
        CdktrCli::Ui => {
            let _ = tui_main().await;
            ()
        }
        CdktrCli::Task(args) => (),
    }
}
