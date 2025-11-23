/// Store for log viewer modal state
use crate::actions::Action;
use cdktr_ipc::log_manager::model::LogMessage;
use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputField {
    StartTime,
    EndTime,
    Grep,
}

/// State for the log viewer modal
#[derive(Debug, Clone)]
pub struct LogViewerState {
    /// Whether the log viewer is open
    pub is_open: bool,

    /// The workflow ID being viewed
    pub workflow_id: Option<String>,

    /// Whether in live tail mode (true) or query mode (false)
    pub is_live_mode: bool,

    /// Collected log lines
    pub logs: Vec<String>,

    /// Scroll offset in the log view
    pub scroll_offset: usize,

    /// Start time for query mode (default: 2 days ago)
    pub start_time: DateTime<Utc>,

    /// End time for query mode (default: now)
    pub end_time: DateTime<Utc>,

    /// Whether query is currently loading
    pub is_loading: bool,

    /// Start time input string (editable)
    pub start_time_input: String,

    /// End time input string (editable)
    pub end_time_input: String,

    /// Grep filter input
    pub grep_filter: String,

    /// Currently focused input field
    pub focused_field: Option<InputField>,

    /// Cursor position in the focused input field
    pub cursor_position: usize,

    /// Error message from last query (if any)
    pub error_message: Option<String>,

    /// Whether in editing mode (focused on input fields)
    pub is_editing: bool,

    /// Whether auto-scroll is enabled in live mode
    pub auto_scroll: bool,
}

impl Default for LogViewerState {
    fn default() -> Self {
        let end_time = Utc::now();
        let start_time = end_time - Duration::days(2);

        // Format with millisecond precision
        let start_time_input = start_time.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        let end_time_input = end_time.format("%Y-%m-%d %H:%M:%S%.3f").to_string();

        Self {
            is_open: false,
            workflow_id: None,
            is_live_mode: false, // Default to query mode
            logs: Vec::new(),
            scroll_offset: 0,
            start_time,
            end_time,
            is_loading: false,
            start_time_input,
            end_time_input,
            grep_filter: String::new(),
            focused_field: None,
            cursor_position: 0,
            error_message: None,
            is_editing: false,
            auto_scroll: true,
        }
    }
}

/// Store for log viewer
#[derive(Clone)]
pub struct LogViewerStore {
    state: Arc<RwLock<LogViewerState>>,
}

impl LogViewerStore {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(LogViewerState::default())),
        }
    }

    pub fn get_state(&self) -> LogViewerState {
        self.state.read().unwrap().clone()
    }

    pub fn reduce(&self, action: &Action) {
        let mut state = self.state.write().unwrap();

        match action {
            Action::OpenLogViewer(workflow_id) => {
                state.is_open = true;
                state.workflow_id = Some(workflow_id.clone());
                state.logs.clear();
                state.scroll_offset = 0;
                state.is_live_mode = false;
                state.cursor_position = 0;
                state.error_message = None;
                state.is_editing = false;
            }

            Action::CloseLogViewer => {
                state.is_open = false;
                state.workflow_id = None;
                state.logs.clear();
                state.scroll_offset = 0;
            }

            Action::ToggleLogMode => {
                state.is_live_mode = !state.is_live_mode;
                // Clear logs when switching modes
                state.logs.clear();
                state.scroll_offset = 0;
                state.error_message = None;
                state.is_editing = false;
                state.focused_field = None;
                state.cursor_position = 0;

                // Reset time range to defaults when entering query mode
                if !state.is_live_mode {
                    state.end_time = Utc::now();
                    state.start_time = state.end_time - Duration::days(2);
                    state.start_time_input =
                        state.start_time.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
                    state.end_time_input =
                        state.end_time.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
                    state.grep_filter.clear();
                    state.focused_field = None;
                    state.cursor_position = state.start_time_input.len();
                } else {
                    state.focused_field = None;
                    state.cursor_position = 0;
                }
            }
            Action::ExecuteLogQuery => {
                state.is_loading = true;
                state.logs.clear();
                state.error_message = None;
            }

            Action::QueryLogsResult(logs) => {
                state.is_loading = false;
                state.logs = logs.clone();
                state.scroll_offset = 0;
                state.error_message = None;
            }

            Action::QueryLogsError(err) => {
                state.is_loading = false;
                state.logs = vec![err.clone()];
                state.error_message = Some("Query Error".to_string());
                state.scroll_offset = 0;
            }

            Action::LogReceived(log_msg) => {
                // Only add logs if viewer is open, in live mode, and matches the workflow
                if state.is_open && state.is_live_mode {
                    if let Some(wf_id) = &state.workflow_id {
                        if &log_msg.workflow_id == wf_id {
                            state.logs.push(log_msg.format_full());
                            // Auto-scroll to bottom only if auto_scroll is enabled
                            if state.auto_scroll {
                                state.scroll_offset = 0;
                            } else {
                                state.scroll_offset += 1; // to account for new log to keep in place
                            }
                        }
                    }
                }
            }

            _ => {}
        }
    }

    /// Add a log line (for live tail mode)
    pub fn add_log(&self, log: String) {
        let mut state = self.state.write().unwrap();
        state.logs.push(log);

        // Auto-scroll to bottom only if auto_scroll is enabled
        if state.is_live_mode && state.auto_scroll && !state.logs.is_empty() {
            state.scroll_offset = 0;
        }
    }

    /// Set logs (for query mode)
    pub fn set_logs(&self, logs: Vec<String>) {
        let mut state = self.state.write().unwrap();
        state.logs = logs;
        state.scroll_offset = 0;
    }

    /// Scroll down
    pub fn scroll_down(&self, amount: usize) {
        let mut state = self.state.write().unwrap();
        state.scroll_offset = state.scroll_offset.saturating_add(amount);
        // Disable auto-scroll when user manually scrolls in live mode
        if state.is_live_mode {
            state.auto_scroll = false;
        }
    }

    /// Scroll up
    pub fn scroll_up(&self, amount: usize) {
        let mut state = self.state.write().unwrap();
        state.scroll_offset = state.scroll_offset.saturating_sub(amount);
        // Disable auto-scroll when user manually scrolls in live mode
        if state.is_live_mode {
            state.auto_scroll = false;
        }
    }

    /// Update the currently focused input field
    pub fn update_input(&self, c: char) {
        let mut state = self.state.write().unwrap();
        if let Some(field) = state.focused_field {
            let cursor_pos = state.cursor_position;
            let text = match field {
                InputField::StartTime => &mut state.start_time_input,
                InputField::EndTime => &mut state.end_time_input,
                InputField::Grep => &mut state.grep_filter,
            };

            // Insert character at cursor position
            if cursor_pos <= text.len() {
                text.insert(cursor_pos, c);
                state.cursor_position += 1;
            }
        }
    }

    /// Delete last character from focused input
    pub fn delete_input(&self) {
        let mut state = self.state.write().unwrap();
        if let Some(field) = state.focused_field {
            if state.cursor_position > 0 {
                let cursor_pos = state.cursor_position;
                let text = match field {
                    InputField::StartTime => &mut state.start_time_input,
                    InputField::EndTime => &mut state.end_time_input,
                    InputField::Grep => &mut state.grep_filter,
                };

                text.remove(cursor_pos - 1);
                state.cursor_position -= 1;
            }
        }
    }

    /// Focus next input field (Tab key)
    pub fn focus_next_field(&self) {
        let mut state = self.state.write().unwrap();
        state.focused_field = match state.focused_field {
            None => Some(InputField::StartTime),
            Some(InputField::StartTime) => Some(InputField::EndTime),
            Some(InputField::EndTime) => Some(InputField::Grep),
            Some(InputField::Grep) => Some(InputField::StartTime),
        };

        // Set cursor to end of new field
        if let Some(field) = state.focused_field {
            state.cursor_position = match field {
                InputField::StartTime => state.start_time_input.len(),
                InputField::EndTime => state.end_time_input.len(),
                InputField::Grep => state.grep_filter.len(),
            };
        }
    }

    /// Focus previous input field (Shift+Tab)
    pub fn focus_prev_field(&self) {
        let mut state = self.state.write().unwrap();
        state.focused_field = match state.focused_field {
            None => Some(InputField::Grep),
            Some(InputField::StartTime) => Some(InputField::Grep),
            Some(InputField::EndTime) => Some(InputField::StartTime),
            Some(InputField::Grep) => Some(InputField::EndTime),
        };

        // Set cursor to end of new field
        if let Some(field) = state.focused_field {
            state.cursor_position = match field {
                InputField::StartTime => state.start_time_input.len(),
                InputField::EndTime => state.end_time_input.len(),
                InputField::Grep => state.grep_filter.len(),
            };
        }
    }

    /// Clear focus from input fields
    pub fn clear_focus(&self) {
        let mut state = self.state.write().unwrap();
        state.focused_field = None;
        state.cursor_position = 0;
    }

    pub fn enter_editing_mode(&self) {
        let mut state = self.state.write().unwrap();
        state.is_editing = true;
        // Auto-focus on first field when entering edit mode
        state.focused_field = Some(InputField::StartTime);
        state.cursor_position = state.start_time_input.len();
    }

    pub fn exit_editing_mode(&self) {
        let mut state = self.state.write().unwrap();
        state.is_editing = false;
        state.focused_field = None;
        state.cursor_position = 0;
    }

    /// Move cursor left
    pub fn cursor_left(&self) {
        let mut state = self.state.write().unwrap();
        if state.cursor_position > 0 {
            state.cursor_position -= 1;
        }
    }

    /// Move cursor right
    pub fn cursor_right(&self) {
        let mut state = self.state.write().unwrap();
        if let Some(field) = state.focused_field {
            let max_len = match field {
                InputField::StartTime => state.start_time_input.len(),
                InputField::EndTime => state.end_time_input.len(),
                InputField::Grep => state.grep_filter.len(),
            };

            if state.cursor_position < max_len {
                state.cursor_position += 1;
            }
        }
    }

    /// Parse input strings to DateTime and update state
    pub fn set_error(&self, error: String) {
        let mut state = self.state.write().unwrap();
        state.error_message = Some(error);
        state.logs.clear();
    }

    pub fn clear_error(&self) {
        let mut state = self.state.write().unwrap();
        state.error_message = None;
    }

    pub fn parse_time_inputs(&self) -> Result<(), String> {
        let mut state = self.state.write().unwrap();

        // Try to parse start time
        let start = chrono::DateTime::parse_from_str(
            &format!("{} +00:00", state.start_time_input),
            "%Y-%m-%d %H:%M:%S%.3f %z",
        )
        .map_err(|e| format!("Invalid start time: {}", e))?;

        // Try to parse end time
        let end = chrono::DateTime::parse_from_str(
            &format!("{} +00:00", state.end_time_input),
            "%Y-%m-%d %H:%M:%S%.3f %z",
        )
        .map_err(|e| format!("Invalid end time: {}", e))?;

        state.start_time = start.with_timezone(&Utc);
        state.end_time = end.with_timezone(&Utc);

        Ok(())
    }

    /// Toggle auto-scroll setting
    /// In live mode, this controls whether new logs auto-scroll the view
    pub fn toggle_auto_scroll(&self) {
        let mut state = self.state.write().unwrap();
        state.auto_scroll = !state.auto_scroll;
    }

    /// Get filtered logs based on grep pattern (regex)
    pub fn get_filtered_logs(&self) -> Vec<String> {
        let state = self.state.read().unwrap();

        if state.grep_filter.is_empty() {
            return state.logs.clone();
        }

        // Try to compile as regex, fall back to literal string if invalid
        match Regex::new(&state.grep_filter) {
            Ok(re) => state
                .logs
                .iter()
                .filter(|log| re.is_match(log))
                .cloned()
                .collect(),
            Err(_) => {
                // Fall back to case-insensitive literal match if regex is invalid
                let filter_lower = state.grep_filter.to_lowercase();
                state
                    .logs
                    .iter()
                    .filter(|log| log.to_lowercase().contains(&filter_lower))
                    .cloned()
                    .collect()
            }
        }
    }
}
