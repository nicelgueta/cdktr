mod executors;
mod models;
use cdktr_core::exceptions::GenericError;
use log::{debug, error, warn};
use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{fs, sync::Mutex};

use models::key_from_path;
pub use models::{FromYaml, Task, WorkFlowDAG, Workflow};

/// BFS traversal of the workflow directory to find all workflows. Will log and skip
/// any items that failed to parse. If none parse, this reutrns an empty hashmap
pub async fn get_yaml_map<T: FromYaml>(workflow_dir: &str) -> HashMap<String, T> {
    let dir = Path::new(workflow_dir).to_owned();
    let mut workflows = HashMap::new();
    let mut dirs_to_scan: VecDeque<PathBuf> = VecDeque::new();
    dirs_to_scan.push_back(dir);

    while dirs_to_scan.len() > 0 {
        let dir = dirs_to_scan.pop_front().unwrap();
        let read_dir = fs::read_dir(&dir).await;
        match read_dir {
            Ok(mut entries) => {
                let mut valid_entries = Vec::new();
                while let Ok(entry) = entries.next_entry().await {
                    if let Some(valid_entry) = entry {
                        valid_entries.push(valid_entry);
                    } else {
                        break; // None means no entries left
                    }
                }
                for entry in valid_entries {
                    let path = entry.path();
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
                            path.to_str().expect("failed to get apth as str"),
                        )
                        .await
                        {
                            Ok(workflow) => workflow,
                            Err(e) => {
                                warn!(
                                    "Parsing failure for {}. Not a valid workflow definition. Original error: {}",
                                    path.display(),
                                    e.to_string()
                                );
                                warn!("Skipping workflow {}", path.display());
                                continue;
                            }
                        };
                        workflows
                            .insert(key_from_path(path, PathBuf::from(workflow_dir)), workflow);
                    } else if path.is_dir() {
                        dirs_to_scan.push_back(path);
                    }
                }
            }
            Err(e) => {
                error!(
                    "Unable to read directory {}: {}",
                    dir.display(),
                    e.to_string()
                );
            }
        }
    }
    workflows
}

#[derive(Debug, Clone)]
pub struct WorkflowStore {
    dir: String,
    inner: Arc<Mutex<HashMap<String, Workflow>>>,
}
impl WorkflowStore {
    pub async fn from_dir(workflow_dir: &str) -> Result<Self, GenericError> {
        Ok(Self {
            dir: workflow_dir.to_string(),
            inner: Arc::new(Mutex::new(get_yaml_map(workflow_dir).await)),
        })
    }
    pub async fn get(&self, workflow_id: &str) -> Option<Workflow> {
        let inner_mutex = self.inner.lock().await;
        match (*inner_mutex).get(workflow_id) {
            Some(workflow) => Some(workflow.clone()),
            None => None,
        }
    }

    pub fn get_workflow_dir(&self) -> &str {
        self.dir.as_str()
    }

    pub async fn count(&self) -> usize {
        self.inner.lock().await.len()
    }

    pub async fn refresh_workflows(&mut self) {
        let mut inner_mutex = self.inner.lock().await;
        *inner_mutex = get_yaml_map(&self.dir).await;
        debug!(
            "Workflow store refreshed with {} workflows",
            inner_mutex.len()
        )
    }
    pub async fn to_string(&self) -> String {
        let inner_mutex = self.inner.lock().await;
        let workflows = inner_mutex.clone();
        serde_json::to_string(&workflows).expect("Workflow store could not be serialised to JSON")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::{TempDir, tempdir};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct MockYamlContent {
        name: String,
    }
    #[async_trait::async_trait]
    impl FromYaml for MockYamlContent {
        type Error = GenericError;
        async fn from_yaml(file_path: &str) -> Result<Self, Self::Error> {
            let obj: MockYamlContent =
                serde_norway::from_str(&tokio::fs::read_to_string(file_path).await.unwrap())
                    .unwrap();
            Ok(obj)
        }
    }

    #[test]
    fn test_key_from_path() {
        let p = PathBuf::from("/some/absolute/path/example.file");
        let wf_dir = PathBuf::from("/some/absolute");
        assert_eq!(key_from_path(p, wf_dir), "path.example")
    }

    fn get_tmp_dir() -> (std::path::PathBuf, TempDir) {
        let tmp_dir = tempdir().unwrap();
        let root_path = tmp_dir.path().join(PathBuf::from("root/workflow_dir/"));

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
        (root_path, tmp_dir)
    }

    #[tokio::test]
    async fn test_get_workflow_map_with_nested_yaml_files() {
        let (wf_dir, _tmp_dir) = get_tmp_dir();

        // Call function
        let result = get_yaml_map::<MockYamlContent>(wf_dir.to_str().unwrap()).await;

        let mut expected = HashMap::new();
        expected.insert(
            "workflow1".to_string(),
            MockYamlContent {
                name: "wf1".to_string(),
            },
        );
        expected.insert(
            "sub1.workflow2".to_string(),
            MockYamlContent {
                name: "wf2".to_string(),
            },
        );
        expected.insert(
            "sub1.sub2.workflow3".to_string(),
            MockYamlContent {
                name: "wf3".to_string(),
            },
        );

        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_no_file_descriptor_leak_on_multiple_refreshes() {
        // This test simulates the production scenario where workflows are refreshed
        // every 60 seconds. We perform many rapid refreshes to ensure file descriptors
        // are properly released with async tokio::fs instead of blocking std::fs.

        let (wf_dir, _tmp_dir) = get_tmp_dir();

        // Simulate 100 rapid refreshes (much more aggressive than production)
        for i in 0..100 {
            let result = get_yaml_map::<MockYamlContent>(wf_dir.to_str().unwrap()).await;

            // Verify we still get correct results
            assert_eq!(result.len(), 3, "Failed on iteration {}", i);
            assert!(result.contains_key("workflow1"));
            assert!(result.contains_key("sub1.workflow2"));
            assert!(result.contains_key("sub1.sub2.workflow3"));
        }

        // If we've made it here without "too many open files" errors (errno 24),
        // the file descriptors are being properly released
    }

    #[tokio::test]
    async fn test_workflow_store_refresh_no_fd_leak() {
        // Test the WorkflowStore refresh_workflows method specifically
        // Use the real test_artifacts directory with actual workflow files
        let test_dir = "./test_artifacts/workflows";

        let mut store = WorkflowStore::from_dir(test_dir).await.unwrap();
        let initial_count = store.count().await;

        // Simulate multiple refresh cycles
        for i in 0..50 {
            store.refresh_workflows().await;
            let count = store.count().await;
            assert_eq!(count, initial_count, "Failed on refresh iteration {}", i);
        }

        // Success means no file descriptor leaks
    }
}
