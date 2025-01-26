use clap::Parser;
use dotenv::dotenv;
use log::{error, info};
use models::InstanceType;
use std::env;

use cdktr_core::instance::{start_agent, start_principal};
use cdktr_tui::tui_main;
use rustyrs::get_slug;

mod api;
mod models;

#[derive(Parser)]
#[command(name = "cdktr")]
#[command(bin_name = "cdktr")]
enum CdktrCli {
    Tui,
    // Task(TaskArgs),
    Start(StartArgs),
}

#[derive(clap::Args)]
#[command(version, about, long_about = None)]
struct TaskArgs {
    #[arg(long, short)]
    json: Option<std::path::PathBuf>,
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
            let instance_id = get_slug(3).unwrap();
            info!(
                "Starting {} instance: {}",
                instance_type.to_string(),
                &instance_id
            );
            match instance_type {
                InstanceType::AGENT => {
                    start_agent(instance_id, principal_host, principal_port, max_tasks).await
                }
                InstanceType::PRINCIPAL => {
                    let database_url: Option<String> = None;
                    start_principal(principal_host, principal_port, database_url).await
                }
            }
        }
        CdktrCli::Tui => {
            let _ = tui_main().await;
            ()
        }
    }
}
