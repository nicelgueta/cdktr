use clap::Parser;
use dotenv::dotenv;
use log::{error, info};
use models::InstanceType;
use std::env;

use cdktr_ipc::instance::{start_agent, start_principal};
use cdktr_tui::tui_main;
use rustyrs::get_slug;

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

    #[arg(long, short, default_value_t = 5)]
    max_tasks: usize,

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

    let principal_host = env::var("CDKTR_PRINCIPAL_HOST").unwrap_or("0.0.0.0".to_string());

    let principal_port: usize = match env::var("CDKTR_PRINCIPAL_PORT") {
        Ok(port) => port
            .parse()
            .expect("CDKTR_PRINCIPAL_PORT must be a valid number"),
        Err(_) => {
            error!("Environment variable CDKTR_PRINCIPAL_PORT not set");
            return;
        }
    };

    match cli_instance {
        CdktrCli::Start(args) => {
            let instance_type = args.instance_type;
            let max_tasks = args.max_tasks;
            let instance_id = get_slug(2).unwrap();
            info!(
                "Starting {} instance: {}",
                instance_type.to_string(),
                &instance_id
            );
            match instance_type {
                InstanceType::AGENT => {
                    start_agent(instance_id, principal_host, principal_port, max_tasks).await
                }
                InstanceType::PRINCIPAL => start_principal(principal_host, principal_port).await,
            }
        }
        CdktrCli::Ui => {
            let _ = tui_main().await;
            ()
        }
        CdktrCli::Task(args) => (),
    }
}
