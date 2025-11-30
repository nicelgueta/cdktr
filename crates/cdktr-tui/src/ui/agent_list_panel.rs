/// Agent list panel for displaying registered agents
use crate::actions::PanelId;
use crate::stores::ui_store::UIState;
use cdktr_api::models::AgentInfo;
use cdktr_core::get_cdktr_setting;
use chrono::{Local, TimeZone};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, Widget},
};

pub struct AgentListPanel {
    agents: Vec<AgentInfo>,
    is_focused: bool,
    agent_timeout_ms: usize,
}

impl AgentListPanel {
    pub fn new(agents: Vec<AgentInfo>, ui_state: &UIState) -> Self {
        let is_focused = ui_state.focused_panel == PanelId::MainPanel;

        Self {
            agents,
            is_focused,
            agent_timeout_ms: get_cdktr_setting!(CDKTR_AGENT_HEARTBEAT_TIMEOUT_MS, usize),
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.is_focused {
            Color::Cyan
        } else {
            Color::White
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Registered Agents ")
            .border_style(Style::default().fg(border_color));

        if self.agents.is_empty() {
            block.render(area, buf);
            return;
        }

        // Create header row
        let header = Row::new(vec![
            Cell::from("Agent ID").style(Style::default().fg(Color::Yellow)),
            Cell::from("Status").style(Style::default().fg(Color::Yellow)),
            Cell::from("Running Workflows").style(Style::default().fg(Color::Yellow)),
            Cell::from("Last Ping").style(Style::default().fg(Color::Yellow)),
        ])
        .height(1)
        .style(Style::default().add_modifier(Modifier::BOLD));

        // Create data rows
        let now = chrono::Utc::now().timestamp();
        let rows: Vec<Row> = self
            .agents
            .iter()
            .map(|agent| {
                let last_ping_micros = agent.last_ping_timestamp;
                let last_ping_secs = last_ping_micros / 1_000_000;
                let age_secs = now - last_ping_secs;

                // Status: LOST if no ping within half the timeout
                let (status, status_color) = if age_secs < (self.agent_timeout_ms / 2 / 1000) as i64
                {
                    if agent.running_tasks > 0 {
                        ("RUNNING", Color::LightCyan)
                    } else {
                        ("READY", Color::Green)
                    }
                } else {
                    ("LOST", Color::Red)
                };

                // Format last ping time
                let last_ping_dt = Local.timestamp_opt(last_ping_secs, 0).unwrap();
                let last_ping_str = if age_secs < 60 {
                    format!("{}s ago", age_secs)
                } else {
                    last_ping_dt.format("%H:%M:%S").to_string()
                };

                Row::new(vec![
                    Cell::from(agent.agent_id.to_string()),
                    Cell::from(status).style(Style::default().fg(status_color)),
                    Cell::from(agent.running_tasks.to_string()).style(Style::default().fg(
                        if agent.running_tasks > 0 {
                            Color::Cyan
                        } else {
                            Color::White
                        },
                    )),
                    Cell::from(last_ping_str),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(40), // Agent ID
                Constraint::Percentage(15), // Status
                Constraint::Percentage(20), // Running Tasks
                Constraint::Percentage(25), // Last Ping
            ],
        )
        .header(header)
        .block(block);

        Widget::render(table, area, buf);
    }
}
