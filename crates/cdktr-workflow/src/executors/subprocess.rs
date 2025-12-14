use async_trait::async_trait;
use cdktr_core::models::{FlowExecutionResult, traits};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc::Sender,
};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct SubprocessTask {
    pub cmd: String,
    pub args: Vec<String>,
}

#[async_trait]
impl traits::Executor for SubprocessTask {
    async fn run(
        &self,
        stdout_tx: Sender<String>,
        stderr_tx: Sender<String>,
    ) -> FlowExecutionResult {
        let mut cmd = Command::new(&self.cmd);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.args(self.args.clone());

        let child_process = cmd.spawn();

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
