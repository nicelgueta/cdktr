use models::{FromYaml, Workflow};
use std::{
    collections::VecDeque,
    fs, io,
    path::{Path, PathBuf},
};
mod models;

/// BFS traversal of the workflow directory to find all workflows. Will result in error
/// for any yaml files that were unsuccessfully parsed.
pub fn get_yamls<T: FromYaml>(workflow_dir: &str) -> Result<Vec<T>, io::Error> {
    let dir = Path::new(workflow_dir).to_owned();
    let mut workflows = Vec::new();
    let mut dirs_to_scan: VecDeque<PathBuf> = VecDeque::new();
    dirs_to_scan.push_back(dir);

    while dirs_to_scan.len() > 0 {
        let dir = dirs_to_scan.pop_front().unwrap();
        match fs::read_dir(dir) {
            Ok(entries) => {
                for entry_result in entries {
                    if let Ok(entry) = entry_result {
                        let path = entry.path();
                        dbg!(path.ends_with(".yml"));
                        if path.is_file()
                            && ["yaml", "yml"].contains(
                                &path
                                    .extension()
                                    .expect("Unable to acquire file extension")
                                    .to_str()
                                    .expect("Extension to str yielded None"),
                            )
                        {
                            let workflow = match T::from_yaml(
                                path.to_str().expect("failed to get apth as str")
                            ) {
                                Ok(workflow) => workflow,
                                Err(e) => return Err(io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    format!(
                                        "Parsing failure for {}. Not a valid workflow definition. Original error: {}",
                                        path.display(),
                                        e.to_string()
                                )))
                            };
                            workflows.push(workflow);
                        } else if path.is_dir() {
                            dirs_to_scan.push_back(path);
                        }
                    }
                }
            }
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Unable to read workflow directory: {}", e.to_string()),
                ))
            }
        }
    }
    Ok(workflows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    struct MockWorkflow {
        name: String,
    }
    impl FromYaml for MockWorkflow {
        type Error = io::Error;
        fn from_yaml(file_path: &str) -> Result<Self, Self::Error> {
            let contents = fs::read_to_string(file_path)?;
            let mock_workflow: MockWorkflow = serde_yml::from_str(&contents).unwrap();
            Ok(mock_workflow)
        }
    }

    #[test]
    fn test_get_workflows_with_nested_yaml_files() {
        let tmp_dir = tempdir().unwrap();
        let root_path = tmp_dir.path();

        // Create nested directory structure
        let sub1 = root_path.join("sub1");
        let sub2 = root_path.join("sub1/sub2");
        fs::create_dir_all(&sub2).unwrap();

        // Create .yaml files
        let wf1_path = root_path.join("workflow1.yaml");
        let wf2_path = sub1.join("workflow2.yml");
        let wf3_path = sub2.join("workflow3.yaml");

        File::create(&wf1_path)
            .unwrap()
            .write_all(b"name: wf1")
            .unwrap();
        File::create(&wf2_path)
            .unwrap()
            .write_all(b"name: wf2")
            .unwrap();
        File::create(&wf3_path)
            .unwrap()
            .write_all(b"name: wf3")
            .unwrap();

        // Call function
        let mut result: Vec<MockWorkflow> = get_yamls(root_path.to_str().unwrap()).unwrap();

        // Sort for consistent testing
        result.sort_by(|a, b| a.name.cmp(&b.name));

        let expected = vec![
            MockWorkflow {
                name: "wf1".to_string(),
            },
            MockWorkflow {
                name: "wf2".to_string(),
            },
            MockWorkflow {
                name: "wf3".to_string(),
            },
        ];

        assert_eq!(result, expected);
    }
}
