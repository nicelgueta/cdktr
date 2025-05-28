use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::path::Path;
use std::{fs, io};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct SubprocessTask {
    cmd: String,
    args: Vec<String>,
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct PythonTask {
    extra_pip_packages: Vec<String>,
    sysexe: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum TaskConfig {
    Subprocess(SubprocessTask),
    Python(PythonTask),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Task {
    name: String,
    description: String,
    depends: Option<Vec<String>>,
    config: TaskConfig,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct InnerWorkflow {
    cron: String,
    start_time: String,
    tasks: HashMap<String, Task>,
}

pub trait FromYaml: Sized {
    type Error: Display;
    fn from_yaml(file_path: &str) -> Result<Self, Self::Error>;
}

#[derive(Debug, PartialEq)]
pub struct Workflow {
    name: String,
    path: String,
    inner: InnerWorkflow,
}
impl FromYaml for Workflow {
    type Error = io::Error;
    fn from_yaml(file_path: &str) -> Result<Self, io::Error> {
        let file = Path::new(file_path);
        let contents = fs::read_to_string(file)?;
        let name = file
            .file_name()
            .expect("Failed to get name from yaml file")
            .to_str()
            .expect("Failed to convert OsStr to &str")
            .to_string();
        Ok(Self::new(file_path.to_string(), name, &contents))
    }
}
impl Workflow {
    fn new(path: String, name: String, contents: &str) -> Self {
        let inner: InnerWorkflow = serde_yml::from_str(contents).expect("Unable to parse");
        Self { name, path, inner }
    }

    pub fn get_tasks(&self) -> &HashMap<String, Task> {
        &self.inner.tasks
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn path(&self) -> &String {
        &self.path
    }

    pub fn start_time_utc(&self) -> Result<chrono::DateTime<chrono::Utc>, io::Error> {
        let res = chrono::DateTime::parse_from_rfc3339(&self.inner.start_time);
        if let Ok(date) = res {
            Ok(date.to_utc())
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Start time is not a valid ISO 8601 datetime",
            ))
        }
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
  task2:
    name: Task 2
    description: Runs second task
    depends: ["task1"]
    config:
      !Python
      extra_pip_packages:
        - pandas>=2.0.0, < 2.2.0
      sysexe: /usr/bin/python

        "#;
        let workflow = Workflow::new(
            "fake/path/my_workflow.yml".to_string(),
            "my_workflow".to_string(),
            yaml,
        );

        assert_eq!(
            "echo".to_string(),
            match &workflow.get_tasks().get("task1").unwrap().config {
                TaskConfig::Subprocess(cfg) => cfg.cmd.clone(),
                _ => panic!("Wrong enum type"),
            }
        );

        assert_eq!(
            vec!["hello", "world"],
            match &workflow.get_tasks().get("task1").unwrap().config {
                TaskConfig::Subprocess(cfg) => cfg.args.clone(),
                _ => panic!("Wrong enum type"),
            }
        );

        assert_eq!(
            vec!["pandas>=2.0.0, < 2.2.0"],
            match &workflow.get_tasks().get("task2").unwrap().config {
                TaskConfig::Python(cfg) => cfg.extra_pip_packages.clone(),
                _ => panic!("Wrong enum type"),
            }
        );

        assert_eq!(
            "/usr/bin/python",
            match &workflow.get_tasks().get("task2").unwrap().config {
                TaskConfig::Python(cfg) => cfg.sysexe.clone().unwrap(),
                _ => panic!("Wrong enum type"),
            }
        );

        assert_eq!(
            chrono::DateTime::from_timestamp(1737376200, 0)
                .unwrap()
                .to_utc(),
            workflow.start_time_utc().unwrap()
        )
    }
}
