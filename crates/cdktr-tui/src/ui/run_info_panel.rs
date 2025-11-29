/// Detail panel for displaying recent workflow statuses
use crate::actions::PanelId;
use crate::stores::ui_store::UIState;
use cdktr_api::models::WorkflowStatusUpdate;
use chrono::{DateTime, Local, TimeZone};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use regex::Regex;

pub struct RunInfoPanel {
    pub recent_statuses: Vec<WorkflowStatusUpdate>,
    pub is_focused: bool,
    pub scroll_state: TableState,
    pub filter_input: String,
}

impl RunInfoPanel {
    pub fn new(
        recent_statuses: Vec<WorkflowStatusUpdate>,
        ui_state: &UIState,
        filter_input: String,
        _scroll_offset: usize,
    ) -> Self {
        let scroll_state = TableState::default();
        // scroll_state.select(Some(scroll_offset));

        Self {
            recent_statuses,
            is_focused: ui_state.focused_panel == PanelId::RunInfoPanel,
            scroll_state,
            filter_input,
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.is_focused {
            Color::Cyan
        } else {
            Color::White
        };

        // Apply regex filter
        let filtered_statuses: Vec<&WorkflowStatusUpdate> = if self.filter_input.is_empty() {
            self.recent_statuses.iter().collect()
        } else {
            match Regex::new(&self.filter_input) {
                Ok(regex) => self
                    .recent_statuses
                    .iter()
                    .filter(|status| {
                        regex.is_match(status.workflow_id())
                            || regex.is_match(status.workflow_instance_id())
                    })
                    .collect(),
                Err(_) => self.recent_statuses.iter().collect(), // Invalid regex, show all
            }
        };

        // Create title with inline filter
        let title = if self.filter_input.is_empty() {
            format!(
                " Recent Workflow Runs ({}/{}) [Filter: _] ",
                filtered_statuses.len(),
                self.recent_statuses.len()
            )
        } else {
            format!(
                " Recent Workflow Runs ({}/{}) [Filter: {}] ",
                filtered_statuses.len(),
                self.recent_statuses.len(),
                self.filter_input
            )
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color));

        // Create table header
        let header = Row::new(vec!["Workflow ID", "Instance ID", "Status", "Last Updated"]).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

        // Create table rows from filtered statuses
        let rows: Vec<Row> = filtered_statuses
            .iter()
            .map(|status| {
                let status_str = status.status();
                let status_color = match status_str {
                    s if s == "RUNNING" => Color::Cyan,
                    s if s == "COMPLETED" => Color::Green,
                    s if s == "FAILED" || s == "CRASHED" => Color::Red,
                    _ => Color::Yellow,
                };

                // Convert timestamp to datetime
                let timestamp_ms = status.timestamp_ms() as i64;
                let dt: DateTime<Local> = Local.timestamp_millis_opt(timestamp_ms).unwrap();
                let formatted_time = dt.format("%Y-%m-%d %H:%M:%S").to_string();

                Row::new(vec![
                    Cell::from(status.workflow_id()),
                    Cell::from(status.workflow_instance_id()),
                    Cell::from(status_str).style(Style::default().fg(status_color)),
                    Cell::from(formatted_time),
                ])
            })
            .collect();

        // Create table with column widths and scrolling
        let table = Table::new(
            rows,
            [
                Constraint::Percentage(25), // Workflow ID
                Constraint::Percentage(25), // Instance ID
                Constraint::Percentage(20), // Status
                Constraint::Percentage(30), // Last Updated
            ],
        )
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().bg(Color::DarkGray));

        ratatui::widgets::StatefulWidget::render(table, area, buf, &mut self.scroll_state.clone());
    }
}
