
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
    pub instance_id: String,
    pub command: String,
    pub args: Option<Vec<String>>
}
impl Task {
    pub fn to_msg_string(&self) -> String {
        match &self.args {
            Some(args) => {
                let arg_str = args.join("|");
                format!("{}|{}|{}", self.instance_id, self.command, arg_str)
            },
            None => format!("{}|{}", self.instance_id, &self.command)
        }
    }
    pub fn from_zmq_args(args: Vec<String>) -> Self {
        // TODO: change panics to Result
        if args.len() < 3 {
            panic!("ZMQ args must be at least 3 args to create task")
        };
        if args[0] != "TASKDEF".to_string() {
            panic!("Does not appear to be a valid TASKDEF. Missing `TASKDEF`")
        };
        let cmd_args = if args.len() > 3 {
            Some(args[2..].iter().map(|x|x.to_string()).collect())
        } else {
            None
        };
        Self {
            instance_id: args[0].clone(),
            command: args[1].clone(),
            args: cmd_args
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
            instance_id: "ID2".to_string(),
            command: "echo".to_string(),
            args: Some(vec!["World".to_string(), "something".to_string()])
        };
        assert_eq!(task.to_msg_string(), "ID2|echo|World|something".to_string())
    }
    #[test]
    fn task_to_msg_string_no_args(){
        let task = Task {
            instance_id: "ID3".to_string(),
            command: "echo".to_string(),
            args: None
        };
        assert_eq!(task.to_msg_string(), "ID3|echo".to_string())
    }

    #[test]
    fn test_task_from_zmq_args(){
        let zmqargs: Vec<String> = vec![
            "TASKDEF", "INSID", "CMD", "ARG1", "ARG2"
        ].iter().map(|x|x.to_string()).collect();
        let _task = Task::from_zmq_args(zmqargs);
    }
}