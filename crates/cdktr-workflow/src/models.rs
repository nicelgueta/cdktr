use async_trait::async_trait;
use cdktr_core::exceptions::GenericError;
use cdktr_core::get_cdktr_setting;
use cdktr_core::models::{FlowExecutionResult, traits};
use daggy::{self, Dag, NodeIndex, Walker};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::{process::Command, sync::mpsc::Sender};

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
    description: Option<String>,
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
    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct InnerWorkflow {
    name: String,
    cron: Option<String>,
    description: Option<String>,
    start_time: Option<String>,
    tasks: HashMap<String, Task>,
}
impl InnerWorkflow {
    /// Checks for cycles and returns a WorkFlowDAG. Returns
    /// error if dag cannot be constructed owing to cycles
    fn gen_dag(&self, name: &str) -> Result<WorkFlowDAG, GenericError> {
        WorkFlowDAG::from_tasks(name.to_string(), &self.tasks)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkFlowDAG {
    name: String,
    task_id_node_ix_map: HashMap<String, NodeIndex<u32>>,
    task_map: HashMap<String, Task>,
    inner: Dag<String, u32>,
    first_tasks: HashSet<String>,
}
impl WorkFlowDAG {
    fn from_tasks(name: String, tasks: &HashMap<String, Task>) -> Result<Self, GenericError> {
        let mut task_id_node_ix_map = HashMap::new();
        let mut inner: Dag<String, u32> = Dag::new();
        let mut first_tasks = HashSet::new();
        for (task_id, task) in tasks {
            if !task_id_node_ix_map.contains_key(task_id) {
                let node_index = inner.add_node(task_id.clone());
                task_id_node_ix_map.insert(task_id.to_string(), node_index);
            };
            match task.get_dependencies() {
                Some(deps) => {
                    if deps.is_empty() {
                        // task has empty deps so is top
                        first_tasks.insert(task_id.to_string());
                    }
                    for dep in deps {
                        let node_index = if !task_id_node_ix_map.contains_key(&dep) {
                            let node_index = inner.add_node(dep.clone());
                            task_id_node_ix_map.insert(dep.clone(), node_index);
                            node_index
                        } else {
                            task_id_node_ix_map.get(&dep).unwrap().clone()
                        };
                        if let Err(e) = inner.add_edge(
                            node_index,
                            task_id_node_ix_map.get(task_id).unwrap().clone(),
                            0,
                        ) {
                            return Err(GenericError::WorkflowError(format!(
                                "Invalid Workflow. DAG edge '{}'->'{}' causes a cycle. Error: {}",
                                dep,
                                task_id,
                                e.to_string()
                            )));
                        }
                    }
                }
                None => {
                    first_tasks.insert(task_id.to_string());
                }
            }
        }
        Ok(Self {
            name,
            task_id_node_ix_map,
            task_map: tasks.clone(),
            inner,
            first_tasks,
        })
    }

    pub fn get_first_tasks(&self) -> Vec<String> {
        self.first_tasks
            .iter()
            .map(|x| x.clone())
            .collect::<Vec<String>>()
    }

    pub fn get_task(&self, task_id: &str) -> Option<&Task> {
        self.task_map.get(task_id)
    }

    pub fn get_dependents(&self, task_id: &str) -> Result<Vec<&String>, GenericError> {
        let nix = if let Some(ix) = self.task_id_node_ix_map.get(task_id) {
            ix
        } else {
            return Err(GenericError::RuntimeError(format!(
                "task id {} does not exist",
                task_id
            )));
        };
        let mut deps = Vec::new();
        let mut walker = self.inner.children(*nix);
        while let Some((edge_i, node_i)) = walker.walk_next(&self.inner) {
            deps.push(
                self.inner
                    .node_weight(node_i)
                    .expect("Should have a node if iterating over dag"),
            )
        }
        Ok(deps)
    }

    pub fn node_count(&self) -> usize {
        self.inner.node_count()
    }
}

pub trait FromYaml: Sized {
    type Error: Display;
    fn from_yaml(file_path: &str) -> Result<Self, Self::Error>;
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Workflow {
    id: String,
    name: String,
    description: Option<String>,
    path: String,
    dag: WorkFlowDAG,
    cron: Option<String>,
    start_time: Option<String>,
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
                    file.to_str(),
                    e.to_string()
                )));
            }
        };
        // let name = key_from_path(file.to_path_buf(), file.parent().unwrap().to_path_buf());

        let workflow = Self::new(file_path.to_string(), &contents)?;
        workflow.validate()?;
        Ok(workflow)
    }
}
impl Workflow {
    pub fn new(path: String, contents: &str) -> Result<Self, GenericError> {
        let inner_res = serde_norway::from_str::<InnerWorkflow>(contents);
        match inner_res {
            Ok(inner) => {
                let dag = inner.gen_dag(&inner.name)?;
                Ok(Self {
                    id: path_to_workflow_id(&path)?,
                    name: inner.name,
                    description: inner.description,
                    path,
                    dag,
                    cron: inner.cron,
                    start_time: inner.start_time,
                })
            }
            Err(e) => Err(GenericError::ParseError(format!(
                "Failed to parse workflow yaml. Error: {}",
                e.to_string()
            ))),
        }
    }

    pub fn get_dag(&self) -> &WorkFlowDAG {
        &self.dag
    }

    pub fn get_task(&self, task_id: &str) -> Option<&Task> {
        self.dag.get_task(task_id)
    }

    // equality values
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn path(&self) -> &String {
        &self.path
    }

    pub fn cron(&self) -> Option<&String> {
        match &self.cron {
            Some(cron) => Some(&cron),
            None => None,
        }
    }
    //

    pub fn start_time_utc(&self) -> Result<chrono::DateTime<chrono::Utc>, GenericError> {
        let start_time = if let Some(t) = &self.start_time {
            t
        } else {
            return Err(GenericError::ParseError(
                "No start_time defined for workflow".to_string(),
            ));
        };
        let res = chrono::DateTime::parse_from_rfc3339(start_time);
        if let Ok(date) = res {
            Ok(date.to_utc())
        } else {
            Err(GenericError::WorkflowError(
                "Start time is not a valid ISO 8601 datetime".to_string(),
            ))
        }
    }

    pub fn to_hashmap(&self) -> HashMap<&'static str, String> {
        let mut hm = HashMap::new();
        hm.insert("name", self.name.clone());
        hm.insert(
            "description",
            self.description.clone().unwrap_or(String::new()),
        );
        hm.insert("path", self.path.clone());
        hm.insert(
            "cron",
            self.cron.clone().unwrap_or("NO SCHEDULE".to_string()),
        );
        hm.insert(
            "start_time",
            self.start_time.clone().unwrap_or("NO SCHEDULE".to_string()),
        );
        hm
    }

    pub fn validate(&self) -> Result<(), GenericError> {
        self.start_time_utc()?;
        Ok(())
    }
}

impl PartialEq for Workflow {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
            && self.name() == other.name()
            && self.path() == other.path()
            && self.cron() == other.cron()
    }
    fn ne(&self, other: &Self) -> bool {
        self.id() != other.id()
            || self.name() != other.name()
            || self.path() != other.path()
            || self.cron() != other.cron()
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

fn path_to_workflow_id(path: &str) -> Result<String, GenericError> {
    let re = Regex::new(r"^(.+)\.[^.]+$").unwrap();
    let workflow_dir = get_cdktr_setting!(CDKTR_WORKFLOW_DIR);
    match re.captures(&path).map(|caps| {
        let mut stem = caps[1].to_string();
        stem = stem.replace(&workflow_dir, "");
        while stem.starts_with('/') | stem.starts_with('.') {
            stem.remove(0);
        }
        stem.replace(['/', '\\'], ".")
    }) {
        Some(id) => Ok(id),
        None => Err(GenericError::ParseError(format!(
            "Failed to create a workflow id for the path: {}. Invalid file path or name.",
            &path
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn test_read_workflow() {
        let yaml = r#"
name: Dummy Flow
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
        let workflow = Workflow::new("fake/path/my_workflow.yml".to_string(), yaml).unwrap();

        assert_eq!(
            "echo".to_string(),
            match &workflow.get_task("task1").unwrap().config {
                ExecutableTask::Subprocess(cfg) => cfg.cmd.clone(),
                _ => panic!("Wrong enum type"),
            }
        );

        assert_eq!(
            vec!["hello", "world"],
            match &workflow.get_task("task1").unwrap().config {
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

    #[test]
    fn test_get_dependents() {
        let dir = env::current_dir().unwrap();
        println!("{:?}", dir.to_string_lossy());
        let wf = Workflow::from_yaml("./test_artifacts/workflows/multi-cmd.yml").unwrap();
        let deps = wf.dag.get_dependents("task1").unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], "task2");

        let mut deps = wf.dag.get_dependents("task2").unwrap();
        deps.sort();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps, vec!["task3", "task4"]);
    }

    #[test]
    fn test_path_to_workflow_id() {
        let cases = vec![
            ("myworkflow.draft.yml", Ok("myworkflow.draft")),
            ("multiple_char-s.yaml", Ok("multiple_char-s")),
            ("photo-processing.yml", Ok("photo-processing")),
            ("/home/user/data_flow.yml", Ok("home.user.data_flow")),
            (r"C:\Users\me\notes.final.yml", Ok("C:.Users.me.notes.final")),
            ("no_extension_file", Err(GenericError::ParseError("Failed to create a workflow id for the path: no_extension_file. Invalid file path or name.".to_string()))),
        ];

        for (input, expected) in cases {
            let result = path_to_workflow_id(input);
            let expected = expected.map(|s| s.to_string());
            assert_eq!(result, expected, "Failed on input: {}", input);
        }
    }
}
