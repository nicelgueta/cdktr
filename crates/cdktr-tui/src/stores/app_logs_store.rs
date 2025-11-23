/// AppLogsStore manages application logs displayed in the Admin tab
use crate::actions::Action;
use crate::logger::LogBuffer;
use std::sync::{Arc, RwLock};

/// Internal state for application logs
#[derive(Debug, Clone)]
pub struct AppLogsState {
    /// Formatted log lines
    pub logs: Vec<String>,

    /// Current scroll position in the log view
    pub scroll_offset: usize,
}

/// Store that holds application log state
pub struct AppLogsStore {
    state: Arc<RwLock<AppLogsState>>,
    log_buffer: LogBuffer,
}

impl AppLogsStore {
    pub fn new(log_buffer: LogBuffer) -> Self {
        Self {
            state: Arc::new(RwLock::new(AppLogsState {
                logs: Vec::new(),
                scroll_offset: 0,
            })),
            log_buffer,
        }
    }

    /// Get a read-only snapshot of the current state
    pub fn get_state(&self) -> AppLogsState {
        self.state.read().unwrap().clone()
    }

    /// Refresh logs from the buffer and handle scroll actions
    pub fn dispatch(&mut self, action: &Action) {
        // Always refresh logs from buffer
        self.refresh_logs();

        // Handle scroll actions (keyboard only sends these when Admin tab is active)
        match action {
            _ => {}
        }
    }

    /// Refresh logs from the buffer
    fn refresh_logs(&self) {
        let logs = self.log_buffer.get_logs();
        let mut state = self.state.write().unwrap();
        state.logs = logs;
    }

    /// Scroll down in the log view
    pub fn scroll_down(&self, amount: usize) {
        let mut state = self.state.write().unwrap();
        state.scroll_offset = state.scroll_offset.saturating_sub(amount);
    }

    /// Scroll up in the log view
    pub fn scroll_up(&self, amount: usize) {
        let mut state = self.state.write().unwrap();
        if state.scroll_offset >= state.logs.len() {
            state.scroll_offset = state.logs.len();
        }
        state.scroll_offset = state.scroll_offset.saturating_add(amount);
    }

    /// Reset scroll to bottom (most recent logs)
    pub fn scroll_to_bottom(&self) {
        let mut state = self.state.write().unwrap();
        state.scroll_offset = 0;
    }
}
