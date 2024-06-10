
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

pub mod traits {
    use tokio::sync::mpsc::Sender;

    use super::FlowExecutionResult;
    use core::future::Future;

    pub trait Executor {
        fn new(command: &str, args: Option<Vec<String>>) -> Self ;
        fn run(self, tx: Sender<String>) -> impl Future<Output = FlowExecutionResult> ;
    }

}