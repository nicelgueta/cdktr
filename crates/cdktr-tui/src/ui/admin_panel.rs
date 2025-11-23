/// Admin panel for viewing application logs
use crate::stores::app_logs_store::AppLogsState;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

pub struct AdminPanel {
    pub logs: Vec<String>,
    pub scroll_offset: usize,
}

impl AdminPanel {
    pub fn from_state(app_logs_state: &AppLogsState) -> Self {
        Self {
            logs: app_logs_state.logs.clone(),
            scroll_offset: app_logs_state.scroll_offset,
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Application Logs ")
            .border_style(Style::default().fg(Color::Cyan));

        if self.logs.is_empty() {
            Paragraph::new("No logs yet...")
                .block(block)
                .style(Style::default().fg(Color::DarkGray))
                .render(area, buf);
            return;
        }

        // Calculate which logs to display based on scroll offset
        let inner_height = area.height.saturating_sub(2) as usize; // Subtract borders
        let total_logs = self.logs.len();

        // Start from the end (most recent) and work backwards
        let start_index = if total_logs > inner_height {
            total_logs.saturating_sub(inner_height + self.scroll_offset)
        } else {
            0
        };

        let end_index = total_logs
            .saturating_sub(self.scroll_offset)
            .max(inner_height.min(total_logs));

        let visible_logs: Vec<Line> = self.logs[start_index..end_index]
            .iter()
            .map(|log| Line::from(log.clone()))
            .collect();

        let scroll_indicator = if total_logs > inner_height {
            format!(" [{}/{}] ", end_index, total_logs)
        } else {
            String::new()
        };

        let title = if scroll_indicator.is_empty() {
            " Application Logs ".to_string()
        } else {
            format!(" Application Logs {} ", scroll_indicator)
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Cyan));

        Paragraph::new(visible_logs)
            .block(block)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}
