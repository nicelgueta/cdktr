use std::process::Command;
use std::process::Stdio;
use std::io::{BufRead, BufReader};

#[derive(Debug, PartialEq)]
pub enum FlowExecutionResult {
    SUCCESS,
    CRASHED(String),
    // ABORTED(String),
    // FAILURE(String),
}
impl FlowExecutionResult {
    fn to_string(self) -> String {
        match self {
            Self::CRASHED(v) => v,
            _ => "".to_string()
            // Self::ABORTED(v) => v,
            // Self::FAILURE(v) => v,
        }
    }
}
pub struct Executor {
    command: String,
    args: Option<Vec<String>>,

}

impl Executor {
    pub fn new(command: &str, args: Option<Vec<String>>) -> Self {
        Self {
            command: command.to_string(), args
        }
    }
    pub fn run<F>(self, mut stream_callback: F) -> FlowExecutionResult 
    where 
        F: FnMut(String)
    {
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
                let reader = BufReader::new(stdout);

                reader
                    .lines()
                    .filter_map(|line| line.ok())
                    .for_each(|line| stream_callback(line));

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
    use super::Executor;

    // #[tokio::test]
    #[test]
    fn test_run_flow() {

        let exec = Executor::new("echo", Some(vec!["Running test_run_flow".to_string()]));
        let exec_result = exec.run(|x|println!("{}", x)).to_string();
        assert_eq!(exec_result, "".to_string())
    }

    // #[tokio::test]
    #[test]
    fn test_run_flow_with_callback() {
        let mut outputs: Vec<String> = Vec::new();
        let exec: Executor = Executor::new("printf", Some(vec!["item1\nitem2\nitem3".to_string()]));
        let callback_closure = |x| outputs.push(x);
        exec.run(callback_closure).to_string();
        assert_eq!(outputs, vec!["item1", "item2", "item3"])
    }

}