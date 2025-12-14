use async_trait::async_trait;
use cdktr_core::models::{FlowExecutionResult, traits};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

mod subprocess;
mod uv_python;

pub use subprocess::SubprocessTask;
pub use uv_python::UvPythonTask;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ExecutableTask {
    Subprocess(SubprocessTask),
    UvPython(UvPythonTask),
}

#[async_trait]
impl traits::Executor for ExecutableTask {
    async fn run(
        &self,
        stdout_tx: Sender<String>,
        stderr_tx: Sender<String>,
    ) -> FlowExecutionResult {
        match &self {
            ExecutableTask::Subprocess(sptask) => sptask.run(stdout_tx, stderr_tx).await,
            ExecutableTask::UvPython(uvptask) => uvptask.run(stdout_tx, stderr_tx).await,
        }
    }
}
