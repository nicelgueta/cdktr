use cdktr::hub::{Hub, InstanceType};
use dotenv::dotenv;
use std::env;

use log::{error, info};

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();
    _main().await;
}

async fn _main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        error!("Needs at least arg (1) of either AGENT or PRINCIPAL and (2) PORT");
        return;
    };
    let typ = InstanceType::from_str(&args[1]);
    let instance_host = env::var("CDKT_INSTANCE_HOST").unwrap_or("0.0.0.0".to_string());
    let principal_host = env::var("CDKTR_PRINCIPAL_HOST").unwrap_or("0.0.0.0".to_string());
    let instance_port: usize = args[2].parse().expect("PORT must be a valid number");
    let database_url: Option<String> = None;
    let max_tm_tasks = 8;

    let principal_port = match typ {
        InstanceType::AGENT => env::var("CDKTR_PRINCIPAL_PORT")
            .expect("env var CDKTR_PRINCIPAL_PORT must be set when spawning an agent instance")
            .parse()
            .expect("Principal port must be a valid port number"),
        InstanceType::PRINCIPAL => instance_port,
    };
    info!(
        "Starting {} instance on {}:{}",
        typ.to_string(),
        &instance_host,
        instance_port
    );
    let mut hub = Hub::from_instance_type(typ);

    // begin main app loop
    hub.start(
        instance_host,
        instance_port,
        principal_host,
        principal_port,
        database_url,
        max_tm_tasks,
    )
    .await
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_main_with_invalid_port() {
        env::set_var("CDKT_INSTANCE_HOST", "127.0.0.1");
        env::set_var("CDKTR_PRINCIPAL_HOST", "127.0.0.1");

        let args = vec![
            "program_name".to_string(),
            "AGENT".to_string(),
            "invalid_port".to_string(),
        ];
        env::set_var("RUST_TEST_ARGS", args.join(" "));

        _main().await;
    }

    #[tokio::test]
    async fn test_main_with_missing_args() {
        env::set_var("CDKT_INSTANCE_HOST", "127.0.0.1");
        env::set_var("CDKTR_PRINCIPAL_HOST", "127.0.0.1");

        let args = vec!["program_name".to_string()];
        env::set_var("RUST_TEST_ARGS", args.join(" "));

        _main().await;
    }
}
