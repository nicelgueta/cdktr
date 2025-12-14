use std::process::Stdio;

use async_trait::async_trait;
use cdktr_core::models::{FlowExecutionResult, traits};
use log::info;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc::Sender,
};

/// Special executor for running python scripts using uv
/// to manage custom package installs and virtualenvs
/// See https://docs.astral.sh/uv/guides/scripts/
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct UvPythonTask {
    pub script_path: String,
    pub is_uv_project: Option<bool>,
    pub packages: Option<Vec<String>>,
    pub uv_path: Option<String>,
    pub working_directory: Option<String>,
}

#[async_trait]
impl traits::Executor for UvPythonTask {
    async fn run(
        &self,
        stdout_tx: Sender<String>,
        stderr_tx: Sender<String>,
    ) -> FlowExecutionResult {
        let uv_executable = match &self.uv_path {
            Some(path) => path.clone(),
            None => "uv".to_string(),
        };

        let mut cmd = Command::new(&uv_executable);
        cmd.arg("run");
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // add packages if not a uv project
        if !self.is_uv_project.unwrap_or(false) {
            match &self.packages {
                Some(pkgs) => {
                    for pkg in pkgs {
                        cmd.arg("--with");
                        cmd.arg(format!("{}", pkg));
                    }
                }
                None => {}
            }
        }

        cmd.arg(&self.script_path);

        if let Some(dir) = &self.working_directory {
            cmd.current_dir(dir);
        }

        let child_process = cmd.spawn();

        info!("Starting UV Python process: {:?}", cmd);

        match child_process {
            Ok(mut child) => {
                // handle process
                let stdout = child.stdout.take().expect("unable to acquire stdout");
                let stderr = child.stderr.take().expect("unable to acquire stderr");
                let mut stdout_reader = BufReader::new(stdout).lines();
                let mut stderr_reader = BufReader::new(stderr).lines();

                while let Some(line) = stdout_reader.next_line().await.unwrap() {
                    stdout_tx.send(line).await.unwrap();
                }
                while let Some(line) = stderr_reader.next_line().await.unwrap() {
                    stderr_tx.send(line).await.unwrap()
                }
                match child.wait().await {
                    Ok(exit_status) => match exit_status.success() {
                        true => FlowExecutionResult::SUCCESS,
                        false => FlowExecutionResult::FAILURE("Process failed".to_string()),
                    },
                    Err(e) => FlowExecutionResult::CRASHED(format!(
                        "Process failed to exit cleanly - {}",
                        e.to_string()
                    )),
                }
            }
            Err(e) => {
                // check for errors starting up the process
                let error_msg = e.to_string();
                FlowExecutionResult::CRASHED(format!(
                    "Failed to start child process: {}",
                    &error_msg
                ))
            }
        }
    }
}
