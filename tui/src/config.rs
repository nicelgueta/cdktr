
pub struct AppConfig {
    pub tabs: Vec<String>
}

impl AppConfig {
    pub fn new() -> Self {
        Self {
            tabs: vec!["DASHBOARD".to_string(), "FLOW MANAGER".to_string()]
        }
    }
}