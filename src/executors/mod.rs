mod process_executor;

use crate::models::{
    Task,
    traits::Executor
};
pub use process_executor::ProcessTask;

pub fn get_executor(task: Task) -> impl Executor {
    match task {
        Task::Process(ptask) => process_executor::ProcessExecutor::new(&ptask.command, ptask.args)
    }
}


#[cfg(test)]
mod tests {
    use super::{get_executor, ProcessTask, Task};
    #[test]
    fn test_get_process_executor() {
        let ptask = ProcessTask {
            command: "echo".to_string(),
            args: Some(vec!["hello world".to_string()])
        };
        let task = Task::Process(ptask);
        get_executor(task);

    }
}