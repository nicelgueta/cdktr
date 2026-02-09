use cdktr_api::{API, PrincipalAPI, models::ClientResponseMessage};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString};
use serde_json::Value as JsonValue;

/// Result returned from Principal API calls
#[pyclass]
pub struct Result {
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub error: Option<String>,
    #[pyo3(get)]
    pub payload: Option<PyObject>,
}

#[pymethods]
impl Result {
    #[new]
    #[pyo3(signature = (success, error=None, payload=None))]
    fn new(success: bool, error: Option<String>, payload: Option<PyObject>) -> Self {
        Self {
            success,
            error,
            payload,
        }
    }

    fn __repr__(&self, py: Python) -> String {
        if self.success {
            let payload_str = match &self.payload {
                Some(payload) => match payload.extract::<String>(py) {
                    Ok(s) => s,
                    Err(_) => "<object>".to_string(),
                },
                None => "None".to_string(),
            };
            format!("Result(success=True, payload={})", payload_str)
        } else {
            format!(
                "Result(success=False, error={})",
                self.error.as_ref().unwrap_or(&"Unknown error".to_string())
            )
        }
    }
}

/// Helper function to convert JSON string to Python object
fn json_to_python(py: Python, json_str: &str) -> PyResult<PyObject> {
    match serde_json::from_str::<JsonValue>(json_str) {
        Ok(value) => json_value_to_python(py, value),
        Err(_) => Ok(PyString::new_bound(py, json_str).into()),
    }
}

/// Recursively convert serde_json::Value to Python object
fn json_value_to_python(py: Python, value: JsonValue) -> PyResult<PyObject> {
    match value {
        JsonValue::Null => Ok(py.None()),
        JsonValue::Bool(b) => Ok(b.into_py(py)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_py(py))
            } else if let Some(u) = n.as_u64() {
                Ok(u.into_py(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_py(py))
            } else {
                Ok(py.None())
            }
        }
        JsonValue::String(s) => Ok(PyString::new_bound(py, &s).into()),
        JsonValue::Array(arr) => {
            let list = PyList::empty_bound(py);
            for item in arr {
                list.append(json_value_to_python(py, item)?)?;
            }
            Ok(list.into())
        }
        JsonValue::Object(obj) => {
            let dict = PyDict::new_bound(py);
            for (key, val) in obj {
                dict.set_item(key, json_value_to_python(py, val)?)?;
            }
            Ok(dict.into())
        }
    }
}

impl Result {
    fn from_response_with_py(py: Python, msg: ClientResponseMessage) -> PyResult<Self> {
        match msg {
            ClientResponseMessage::Success | ClientResponseMessage::Pong => Ok(Result {
                success: true,
                error: None,
                payload: None,
            }),
            ClientResponseMessage::SuccessWithPayload(payload) => Ok(Result {
                success: true,
                error: None,
                payload: Some(json_to_python(py, &payload)?),
            }),
            ClientResponseMessage::ClientError(err)
            | ClientResponseMessage::ServerError(err)
            | ClientResponseMessage::Unprocessable(err)
            | ClientResponseMessage::NetworkError(err) => Ok(Result {
                success: false,
                error: Some(err),
                payload: None,
            }),
        }
    }
}

/// Python wrapper for the Principal API client
#[pyclass]
pub struct Principal {
    host: String,
    port: u16,
}

#[pymethods]
impl Principal {
    #[new]
    #[pyo3(signature = (host="localhost".to_string(), port=5561))]
    fn new(host: String, port: u16) -> Self {
        // Set environment variable for the Rust code to use
        std::env::set_var("CDKTR_PRINCIPAL_HOST", &host);
        std::env::set_var("CDKTR_PRINCIPAL_PORT", port.to_string());
        Self { host, port }
    }

    /// Ping the principal to check if it's online
    fn ping(&self, py: Python) -> PyResult<Result> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(async {
            let api = PrincipalAPI::Ping;
            match api.send().await {
                Ok(msg) => Result::from_response_with_py(py, msg),
                Err(e) => Ok(Result {
                    success: false,
                    error: Some(e.to_string()),
                    payload: None,
                }),
            }
        })
    }

    /// List all workflows in the workflow store
    fn list_workflows(&self, py: Python) -> PyResult<Result> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(async {
            let api = PrincipalAPI::ListWorkflowStore;
            match api.send().await {
                Ok(msg) => Result::from_response_with_py(py, msg),
                Err(e) => Ok(Result {
                    success: false,
                    error: Some(e.to_string()),
                    payload: None,
                }),
            }
        })
    }

    /// Run a workflow by ID
    fn run_workflow(&self, py: Python, workflow_id: String) -> PyResult<Result> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(async {
            let api = PrincipalAPI::RunTask(workflow_id);
            match api.send().await {
                Ok(msg) => Result::from_response_with_py(py, msg),
                Err(e) => Ok(Result {
                    success: false,
                    error: Some(e.to_string()),
                    payload: None,
                }),
            }
        })
    }

    /// Query logs from the database
    #[pyo3(signature = (start_timestamp_ms=None, end_timestamp_ms=None, workflow_id=None, workflow_instance_id=None, verbose=false))]
    fn query_logs(
        &self,
        py: Python,
        start_timestamp_ms: Option<u64>,
        end_timestamp_ms: Option<u64>,
        workflow_id: Option<String>,
        workflow_instance_id: Option<String>,
        verbose: bool,
    ) -> PyResult<Result> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(async {
            let api = PrincipalAPI::QueryLogs(
                end_timestamp_ms,
                start_timestamp_ms,
                workflow_id,
                workflow_instance_id,
                verbose,
            );
            match api.send().await {
                Ok(msg) => Result::from_response_with_py(py, msg),
                Err(e) => Ok(Result {
                    success: false,
                    error: Some(e.to_string()),
                    payload: None,
                }),
            }
        })
    }

    /// Get recent workflow statuses (last 10 workflows)
    fn get_recent_workflow_statuses(&self, py: Python) -> PyResult<Result> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(async {
            let api = PrincipalAPI::GetRecentWorkflowStatuses;
            match api.send().await {
                Ok(msg) => Result::from_response_with_py(py, msg),
                Err(e) => Ok(Result {
                    success: false,
                    error: Some(e.to_string()),
                    payload: None,
                }),
            }
        })
    }

    /// Get list of all registered agents
    fn get_registered_agents(&self, py: Python) -> PyResult<Result> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(async {
            let api = PrincipalAPI::GetRegisteredAgents;
            match api.send().await {
                Ok(msg) => Result::from_response_with_py(py, msg),
                Err(e) => Ok(Result {
                    success: false,
                    error: Some(e.to_string()),
                    payload: None,
                }),
            }
        })
    }

    fn __repr__(&self) -> String {
        format!("Principal(host='{}', port={})", self.host, self.port)
    }
}

/// Python module for cdktr
#[pymodule]
fn cdktr(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Principal>()?;
    m.add_class::<Result>()?;
    Ok(())
}
