use tokio::{process::Command, sync::mpsc::Sender};
use std::process::Stdio;
use tokio::io::{BufReader, AsyncBufReadExt};

use crate::models::{
    traits, FlowExecutionResult
};

pub struct ProcessExecutor {
    command: String,
    args: Option<Vec<String>>,

}

impl traits::Executor for ProcessExecutor {
    fn new(command: &str, args: Option<Vec<String>>) -> Self {
        Self {
            command: command.to_string(), args
        }
    }
    async fn run(self, tx: Sender<String>) -> FlowExecutionResult {
        let mut cmd = Command::new(self.command);
        cmd.stdout(Stdio::piped());

        if let Some(args) = self.args {
            cmd.args(args)
        } else {
            &mut cmd
        };
        let child_process = cmd.spawn();

        match child_process {
            Ok(child) => {
                // handle process 
                let stdout = child.stdout.expect("unable to acquire stdout");
                let mut reader = BufReader::new(stdout).lines();

                while let Some(line) = reader.next_line().await.unwrap(){
                    tx.send(line).await.unwrap();
                }
                FlowExecutionResult::SUCCESS
            },
            Err(e) => {
                // check for errors starting up the process
                let error_msg = e.to_string();
                FlowExecutionResult::CRASHED(
                    format!("Failed to start child process: {}", &error_msg)
                )

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
        let (tx, _rx) = mpsc::channel(32);
        let exec_result = exec.run(tx).await.to_string();
        assert_eq!(exec_result, "".to_string())
    }

    #[tokio::test]
    async fn test_run_flow_with_callback() {
        let exec: ProcessExecutor = ProcessExecutor::new("printf", Some(vec!["item1\nitem2\nitem3".to_string()]));
        let (tx, mut rx) = mpsc::channel(32);
        
        let mut outputs: Vec<String> = Vec::new();

        // have to spawn instead of await  in order to move on to the recv messages since for tx 
        // to go out of scope, `run` would have to have exited
        tokio::spawn(
            async move {exec.run(tx).await}
        ); 
        while let Some(msg) = rx.recv().await {
            outputs.push(msg);
        }
        assert_eq!(outputs, vec!["item1", "item2", "item3"])
    }

}