use cdktr_core::hub::{Hub, InstanceType};
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
    if args.len() < 2 {
        error!("Needs at least arg (1) of either AGENT or PRINCIPAL");
        return;
    };
    let typ = InstanceType::from_str(&args[1]);
    let instance_host = env::var("CDKT_INSTANCE_HOST").unwrap_or("0.0.0.0".to_string());
    let principal_host = env::var("CDKTR_PRINCIPAL_HOST").unwrap_or("0.0.0.0".to_string());
    let database_url: Option<String> = None;
    let max_tm_tasks = 8;

    let principal_port = match env::var("CDKTR_PRINCIPAL_PORT") {
        Ok(port) => port
            .parse()
            .expect("CDKTR_PRINCIPAL_PORT must be a valid number"),
        Err(_) => {
            error!("Environment variable CDKTR_PRINCIPAL_PORT not set");
            return;
        }
    };

    let instance_port: usize = match typ {
        InstanceType::AGENT => {
            if args.len() < 3 {
                error!("Needs a port number as arg (2) if spawning an agent instance");
                return;
            } else {
                args[2]
                    .parse()
                    .expect("Instance port number must be a valid number")
            }
        }
        InstanceType::PRINCIPAL => principal_port,
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
