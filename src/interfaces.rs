
#[derive(Debug, PartialEq)]
pub enum FlowExecutionResult {
    SUCCESS,
    CRASHED(String),
    // ABORTED(String),
    // FAILURE(String),
}
impl FlowExecutionResult {
    pub fn to_string(self) -> String {
        match self {
            Self::CRASHED(v) => v,
            _ => "".to_string()
            // Self::ABORTED(v) => v,
            // Self::FAILURE(v) => v,
        }
    }
}

#[derive(Debug)]
pub struct Task {
    pub command: String,
    pub args: Option<Vec<String>>
}
impl Task {
    pub fn to_msg_string(&self) -> String {
        match &self.args {
            Some(args) => {
                let arg_str = args.join("|");
                format!("{}|{}", self.command, arg_str)
            },
            None => self.command.clone()
        }
    }
}

pub mod traits {
    use tokio::sync::mpsc::Sender;

    use super::FlowExecutionResult;
    use core::future::Future;

    pub trait Executor {
        fn new(command: &str, args: Option<Vec<String>>) -> Self ;
        fn run(self, tx: Sender<String>) -> impl Future<Output = FlowExecutionResult> ;
    }

}


#[cfg(test)]
mod tests {
    use super::Task;

    #[test]
    fn task_to_msg_string_args(){
        let task = Task {
            command: "echo".to_string(),
            args: Some(vec!["World".to_string(), "something".to_string()])
        };
        assert_eq!(task.to_msg_string(), "echo|World|something".to_string())
    }
    #[test]
    fn task_to_msg_string_no_args(){
        let task = Task {
            command: "echo".to_string(),
            args: None
        };
        assert_eq!(task.to_msg_string(), "echo".to_string())
    }
}