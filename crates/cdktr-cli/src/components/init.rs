static EXAMPLE_YAML_CONFIG: &str = r#"# Example CDKTR workflow configuration file
name: Example Workflow
description: Example workflow with multiple tasks and dependencies
cron: "0 */5 * * * *" # every 5 minutes
start_time: 2025-01-20T12:30:00+00:00
tasks:
  task1:
    name: Task 1
    description: Runs first task
    config:
      !Subprocess
      cmd: echo
      args:
        - hello from task 1
  task2:
    name: Task 2
    depends: ["task1"]
    description: Runs second task
    config:
      !Subprocess
      cmd: echo
      args:
        - hello from task 2
"#;

pub static EXAMPLE_DOTENV_CONFIG: &str = r#"# Example CDKTR .env configuration file
CDKTR_AGENT_MAX_CONCURRENCY=10
CDKTR_PRINCIPAL_PORT=5561
CDKTR_WORKFLOW_DIR=./workflows
CDKTR_LOG_LEVEL=info
"#;

fn write_yaml_example() -> std::io::Result<()> {
    if !std::path::Path::new("workflows").exists() {
        std::fs::create_dir("workflows")?;
    }
    let path = std::path::Path::new("./workflows/example_workflow.yml");
    std::fs::write(path, EXAMPLE_YAML_CONFIG)
}

fn write_dotenv_example() -> std::io::Result<()> {
    let path = std::path::Path::new("./.env.example");
    std::fs::write(path, EXAMPLE_DOTENV_CONFIG)
}

pub fn handle_init(args: InitArgs) {
    if !args.no_example {
        if let Err(e) = write_yaml_example() {
            println!("Failed to write example workflow: {}", e);
        } else {
            println!("Example workflow written to ./workflows/example_workflow.yml");
        }
    }
    if !args.no_dotenv {
        if let Err(e) = write_dotenv_example() {
            println!("Failed to write example .env file: {}", e);
        } else {
            println!("Example .env file written to ./.env.example");
        }
    }
    print!(
        "Initialised new CDKTR project at {}\n",
        std::env::current_dir().unwrap().to_str().unwrap()
    );
}

/// Arguments for initializing a baseline project structure
/// with example workflow and .env file
#[derive(clap::Args)]
#[command(version, about, long_about = None)]
pub struct InitArgs {
    #[arg(long, default_value_t = false)]
    pub no_example: bool,

    #[arg(long, default_value_t = false)]
    pub no_dotenv: bool,
}

#[cfg(test)]
mod tests {
    use super::{write_dotenv_example, write_yaml_example};

    #[test]
    fn test_write_yaml_example() {
        let result = write_yaml_example();
        assert!(result.is_ok());
        let path = std::path::Path::new("./workflows/example_workflow.yml");
        assert!(path.exists());
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir("workflows").unwrap();
    }

    #[test]
    fn test_write_dotenv_example() {
        let result = write_dotenv_example();
        assert!(result.is_ok());
        let path = std::path::Path::new("./.env.example");
        assert!(path.exists());
        std::fs::remove_file(path).unwrap();
    }
}
