use async_trait::async_trait;
use cdktr_core::exceptions::GenericError;
use cdktr_core::models::{traits, FlowExecutionResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::{process::Command, sync::mpsc::Sender};
use topological_sort::TopologicalSort;

pub fn key_from_path(path: PathBuf, workflow_dir: PathBuf) -> String {
    path.strip_prefix(workflow_dir)
        .ok()
        .map(|relative_path| {
            relative_path
                .with_extension("") // Remove extension
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join(".")
        })
        .unwrap()
}

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
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct PythonTask {
    pub extra_pip_packages: Vec<String>,
    pub sysexe: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ExecutableTask {
    Subprocess(SubprocessTask),
    // Python(PythonTask),
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
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Task {
    name: String,
    description: String,
    depends: Option<Vec<String>>,
    config: ExecutableTask,
}
impl Task {
    pub fn get_dependencies(&self) -> Option<Vec<String>> {
        self.depends.clone()
    }
    pub fn get_exe_task(&self) -> ExecutableTask {
        self.config.clone()
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn description(&self) -> &str {
        &self.description
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct InnerWorkflow {
    cron: String,
    description: Option<String>,
    start_time: String,
    tasks: HashMap<String, Task>,
}
impl InnerWorkflow {
    /// Check for cycles using topo sort
    fn validate_workflow_dag(&self, name: &str) -> Result<(), GenericError> {
        let mut tp: TopologicalSort<String> = TopologicalSort::new();
        let mut ptasks = HashSet::new();
        let mut visited_deps = HashSet::new();
        for (task_id, task) in &self.tasks {
            ptasks.insert(task_id.clone());
            match task.get_dependencies() {
                Some(deps) => {
                    for dep in deps {
                        visited_deps.insert(dep.clone());
                        tp.add_dependency(dep, task_id.clone());
                    }
                }
                None => (),
            }
        }
        let undefineds = visited_deps.difference(&ptasks);
        let offending_ids: Vec<String> = undefineds.map(|x| x.to_string()).collect();
        if offending_ids.len() > 0 {
            return Err(GenericError::WorkflowError(
                format!(
                    "Invalid workflow DAG. One or more tasks lists dependent tasks that are not defined in the workflow: {}",
                    offending_ids.join(",")
                )
            ));
        }
        let mut last_nodes: Vec<String> = Vec::new();
        while tp.len() > 0 {
            let next_nodes = tp.pop_all();
            if next_nodes.is_empty() {
                // have a cycle if length > 0 but no nodes can be popped
                return Err(GenericError::WorkflowError(
                    format!(
                        "Invalid workflow DAG. Workflow '{}' contains a cycle. Last nodes evaluated were {} but no successor could be determined",
                        name,
                        last_nodes.join(",")
                    )
                ));
            };
            last_nodes = next_nodes;
        }
        Ok(())
    }
}

pub trait FromYaml: Sized {
    type Error: Display;
    fn from_yaml(file_path: &str) -> Result<Self, Self::Error>;
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct Workflow {
    name: String,
    path: String,
    inner: InnerWorkflow,
}
impl FromYaml for Workflow {
    type Error = GenericError;
    fn from_yaml(file_path: &str) -> Result<Self, GenericError> {
        let file = Path::new(file_path);
        let contents = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                return Err(GenericError::WorkflowError(format!(
                    "Error reading yaml file {:?}. Error: {}",
                    file.file_name(),
                    e.to_string()
                )))
            }
        };
        let name = key_from_path(file.to_path_buf(), file.parent().unwrap().to_path_buf());
        let workflow = Self::new(file_path.to_string(), name, &contents)?;
        workflow.validate()?;
        Ok(workflow)
    }
}
impl Workflow {
    pub fn new(path: String, name: String, contents: &str) -> Result<Self, GenericError> {
        let inner_res = serde_yml::from_str(contents);
        match inner_res {
            Ok(inner) => Ok(Self { name, path, inner }),
            Err(e) => Err(GenericError::ParseError(format!(
                "Failed to parse workflow yaml. Error: {}",
                e.to_string()
            ))),
        }
    }

    pub fn get_tasks(&self) -> &HashMap<String, Task> {
        &self.inner.tasks
    }

    pub fn get_task(&self, task_id: &str) -> Option<&Task> {
        self.inner.tasks.get(task_id)
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn path(&self) -> &String {
        &self.path
    }

    pub fn start_time_utc(&self) -> Result<chrono::DateTime<chrono::Utc>, GenericError> {
        let res = chrono::DateTime::parse_from_rfc3339(&self.inner.start_time);
        if let Ok(date) = res {
            Ok(date.to_utc())
        } else {
            Err(GenericError::WorkflowError(
                "Start time is not a valid ISO 8601 datetime".to_string(),
            ))
        }
    }

    pub fn validate(&self) -> Result<(), GenericError> {
        self.start_time_utc()?;
        self.inner.validate_workflow_dag(&self.name)?;
        Ok(())
    }
}

/// for easy parsing when workflows are sent over the wire from principals to agents
impl TryFrom<String> for Workflow {
    type Error = serde_json::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        serde_json::from_str::<Workflow>(&value)
    }
}

impl ToString for Workflow {
    fn to_string(&self) -> String {
        serde_json::to_string(self).expect("Workflow could not be serialised to JSON")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_workflow() {
        let yaml = r#"
cron: "*/2 * * * * *"
start_time: 2025-01-20T12:30:00+00:00
tasks:
  task1:
    name: Task 1
    description: Runs first task
    config:
      !Subprocess
      cmd: echo
      args:
        - hello
        - world
  #task2:
  #  name: Task 2
  #  description: Runs second task
  #  depends: ["task1"]
  #  config:
  #    !Python
  #    extra_pip_packages:
  #      - pandas>=2.0.0, < 2.2.0
  #    sysexe: /usr/bin/python

        "#;
        let workflow = Workflow::new(
            "fake/path/my_workflow.yml".to_string(),
            "my_workflow".to_string(),
            yaml,
        )
        .unwrap();

        assert_eq!(
            "echo".to_string(),
            match &workflow.get_tasks().get("task1").unwrap().config {
                ExecutableTask::Subprocess(cfg) => cfg.cmd.clone(),
                _ => panic!("Wrong enum type"),
            }
        );

        assert_eq!(
            vec!["hello", "world"],
            match &workflow.get_tasks().get("task1").unwrap().config {
                ExecutableTask::Subprocess(cfg) => cfg.args.clone(),
                _ => panic!("Wrong enum type"),
            }
        );

        // assert_eq!(
        //     vec!["pandas>=2.0.0, < 2.2.0"],
        //     match &workflow.get_tasks().get("task2").unwrap().config {
        //         ExecutableTask::Python(cfg) => cfg.extra_pip_packages.clone(),
        //         _ => panic!("Wrong enum type"),
        //     }
        // );

        // assert_eq!(
        //     "/usr/bin/python",
        //     match &workflow.get_tasks().get("task2").unwrap().config {
        //         ExecutableTask::Python(cfg) => cfg.sysexe.clone().unwrap(),
        //         _ => panic!("Wrong enum type"),
        //     }
        // );

        assert_eq!(
            chrono::DateTime::from_timestamp(1737376200, 0)
                .unwrap()
                .to_utc(),
            workflow.start_time_utc().unwrap()
        )
    }
}
