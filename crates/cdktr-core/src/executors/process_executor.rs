use crate::models::{traits, FlowExecutionResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::{process::Command, sync::mpsc::Sender};

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct ProcessTask {
    pub command: String,
    pub args: Option<Vec<String>>,
}

pub struct ProcessExecutor {
    command: String,
    args: Option<Vec<String>>,
}

#[async_trait]
impl traits::Executor for ProcessExecutor {
    fn new(command: &str, args: Option<Vec<String>>) -> Self {
        Self {
            command: command.to_string(),
            args,
        }
    }
    async fn run(
        &self,
        stdout_tx: Sender<String>,
        stderr_tx: Sender<String>,
    ) -> FlowExecutionResult {
        let mut cmd = Command::new(&self.command);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Some(args) = &self.args {
            cmd.args(args)
        } else {
            &mut cmd
        };
        let child_process = cmd.spawn();

        match child_process {
            Ok(child) => {
                // handle process
                let stdout = child.stdout.expect("unable to acquire stdout");
                let stderr = child.stderr.expect("unable to acquire stderr");
                let mut stdout_reader = BufReader::new(stdout).lines();
                let mut stderr_reader = BufReader::new(stderr).lines();

                while let Some(line) = stdout_reader.next_line().await.unwrap() {
                    stdout_tx.send(line).await.unwrap();
                }
                while let Some(line) = stderr_reader.next_line().await.unwrap() {
                    stderr_tx.send(line).await.unwrap()
                }
                FlowExecutionResult::SUCCESS
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

#[cfg(test)]
mod tests {
    use super::ProcessExecutor;
    use crate::models::traits::Executor;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_run_flow() {
        let exec = ProcessExecutor::new("echo", Some(vec!["Running test_run_flow".to_string()]));
        let (tx1, _rx) = mpsc::channel(32);
        let (tx2, _rx) = mpsc::channel(32);
        let exec_result = exec.run(tx1, tx2).await._to_string();
        assert_eq!(exec_result, "".to_string())
    }

    #[tokio::test]
    async fn test_run_flow_with_callback() {
        let exec: ProcessExecutor =
            ProcessExecutor::new("printf", Some(vec!["item1\nitem2\nitem3".to_string()]));
        let (tx1, mut rx1) = mpsc::channel(32);
        let (tx2, _rx) = mpsc::channel(32);

        let mut outputs: Vec<String> = Vec::new();

        // have to spawn instead of await  in order to move on to the recv messages since for tx
        // to go out of scope, `run` would have to have exited
        tokio::spawn(async move { exec.run(tx1, tx2).await });
        while let Some(msg) = rx1.recv().await {
            outputs.push(msg);
        }
        assert_eq!(outputs, vec!["item1", "item2", "item3"])
    }
}
