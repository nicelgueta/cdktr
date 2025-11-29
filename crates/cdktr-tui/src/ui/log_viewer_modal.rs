/// Log viewer modal overlay for viewing workflow logs
use crate::stores::log_viewer_store::{InputField, LogViewerState, LogViewerStore};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};
use time::OffsetDateTime;

pub struct LogViewerModal<'a> {
    state: LogViewerState,
    store: &'a LogViewerStore,
}

impl<'a> LogViewerModal<'a> {
    pub fn new(state: LogViewerState, store: &'a LogViewerStore) -> Self {
        Self { state, store }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.state.is_open {
            return;
        }

        // Use full screen
        let modal_area = area;

        // Clear the area behind the modal
        Clear.render(modal_area, buf);

        // Main modal block
        let mode_str = if self.state.is_live_mode {
            "LIVE TAIL"
        } else {
            "QUERY MODE"
        };

        let workflow_id = self
            .state
            .workflow_id
            .as_ref()
            .map(|id| id.as_str())
            .unwrap_or("Unknown");

        let title = format!(" Logs: {} - {} ", workflow_id, mode_str);

        let block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner_area = block.inner(modal_area);
        block.render(modal_area, buf);

        // Split into input fields (query mode only), logs area, and help footer
        let chunks = if self.state.is_live_mode {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),    // Logs
                    Constraint::Length(1), // Help
                ])
                .split(inner_area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Filters row (with borders)
                    Constraint::Min(1),    // Logs
                    Constraint::Length(1), // Help
                ])
                .split(inner_area)
        };

        // Render input fields and logs based on mode
        if !self.state.is_live_mode {
            self.render_query_filters(chunks[0], buf);
            self.render_logs(chunks[1], buf);
            self.render_help(chunks[2], buf);
        } else {
            self.render_logs(chunks[0], buf);
            self.render_help(chunks[1], buf);
        }

        // Render calendar popups on top if open
        if self.state.start_calendar_open || self.state.end_calendar_open {
            self.render_calendar_popup(area, buf);
        }
    }

    fn render_query_filters(&self, area: Rect, buf: &mut Buffer) {
        // Create horizontal layout for filters and status
        let filter_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // Start time
                Constraint::Percentage(30), // End time
                Constraint::Percentage(20), // Grep filter
                Constraint::Percentage(20), // Status
            ])
            .split(area);

        // Render start time with border
        self.render_bordered_input(
            filter_chunks[0],
            buf,
            "Start",
            &self.state.start_time_input,
            InputField::StartTime,
        );

        // Render end time with border
        self.render_bordered_input(
            filter_chunks[1],
            buf,
            "End",
            &self.state.end_time_input,
            InputField::EndTime,
        );

        // Render grep filter with border
        self.render_bordered_input(
            filter_chunks[2],
            buf,
            "Regex",
            &self.state.grep_filter,
            InputField::Grep,
        );

        // Render status with border
        let status_text = if let Some(ref error) = self.state.error_message {
            format!("⚠ {}", error)
        } else if !self.state.logs.is_empty() {
            "✓ Success".to_string()
        } else {
            "Ready".to_string()
        };

        let status_style = if self.state.error_message.is_some() {
            Style::default().fg(Color::Red)
        } else if !self.state.logs.is_empty() {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Gray)
        };

        let status_block = Block::default()
            .title("Status")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White));

        let status_inner = status_block.inner(filter_chunks[3]);
        status_block.render(filter_chunks[3], buf);

        Paragraph::new(status_text)
            .style(status_style)
            .render(status_inner, buf);
    }

    fn render_bordered_input(
        &self,
        area: Rect,
        buf: &mut Buffer,
        label: &str,
        value: &str,
        field: InputField,
    ) {
        let is_focused = self.state.focused_field == Some(field);
        let is_date_field = matches!(field, InputField::StartTime | InputField::EndTime);

        let border_style = if is_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let title = if is_date_field && is_focused {
            format!("{} [📅 Space: Calendar]", label)
        } else {
            label.to_string()
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner_area = block.inner(area);
        block.render(area, buf);

        let text = if is_focused {
            let cursor_pos = self.state.cursor_position.min(value.len());
            let before = &value[..cursor_pos];
            let after = &value[cursor_pos..];
            format!("{}█{}", before, after)
        } else {
            value.to_string()
        };

        let text_style = if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        Paragraph::new(text)
            .style(text_style)
            .render(inner_area, buf);
    }

    fn render_input_field(
        &self,
        area: Rect,
        buf: &mut Buffer,
        label: &str,
        value: &str,
        field: InputField,
    ) {
        let is_focused = self.state.focused_field == Some(field);

        let style = if is_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let text = if is_focused {
            let cursor_pos = self.state.cursor_position.min(value.len());
            let before = &value[..cursor_pos];
            let after = &value[cursor_pos..];
            format!("{}: {}█{}", label, before, after)
        } else {
            format!("{}: {}", label, value)
        };

        Paragraph::new(text).style(style).render(area, buf);
    }
    fn render_logs(&self, area: Rect, buf: &mut Buffer) {
        // Get filtered logs (applies grep filter if set)
        let filtered_logs = self.store.get_filtered_logs();

        if filtered_logs.is_empty() {
            let message = if self.state.is_live_mode {
                "Waiting for logs... (streaming live)"
            } else if !self.state.grep_filter.is_empty() {
                "No logs match the regex pattern"
            } else {
                "No logs found for the specified time range"
            };

            Paragraph::new(message)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .render(area, buf);
            return;
        }

        // Calculate visible logs based on scroll
        let total_logs = filtered_logs.len();
        let visible_height = area.height as usize;

        // In live mode with no scroll, show the most recent logs
        let (start_idx, end_idx) = if self.state.is_live_mode && self.state.auto_scroll {
            let start = total_logs.saturating_sub(visible_height);
            (start, total_logs)
        } else {
            let start = total_logs.saturating_sub(visible_height + self.state.scroll_offset);
            let end = total_logs.saturating_sub(self.state.scroll_offset);
            (start, end)
        };

        let visible_logs: Vec<Line> = filtered_logs[start_idx..end_idx]
            .iter()
            .map(|log| Line::from(log.clone()))
            .collect();

        // Show scroll position indicator similar to Admin panel
        let scroll_indicator = if total_logs > visible_height {
            format!(" [{}/{}] ", end_idx, total_logs)
        } else {
            String::new()
        };

        let mut text = visible_logs;

        // Add scroll indicator at the top if there are more logs
        if !scroll_indicator.is_empty() {
            text.insert(
                0,
                Line::from(Span::styled(
                    scroll_indicator,
                    Style::default().fg(Color::DarkGray),
                )),
            );
        }

        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }

    fn render_help(&self, area: Rect, buf: &mut Buffer) {
        let help_text = if self.state.start_calendar_open || self.state.end_calendar_open {
            "📅 CALENDAR | ←→:Day | ↑↓:Week | PgUp/PgDn:Month | Enter:Select | Esc:Cancel"
        } else if self.state.is_editing {
            "** EDITING ** | Esc:Exit Edit | Tab:Next Field | Enter:Execute | ←→:Move Cursor | Space:Calendar (date fields)"
        } else if self.state.is_live_mode {
            let auto_scroll_text = if self.state.auto_scroll {
                "s:Auto-scroll (on)"
            } else {
                "s:Auto-scroll (off)"
            };
            &format!(
                "Esc:Close | t:Toggle tail/query mode | {} | PgUp/PgDn:Page",
                auto_scroll_text
            )
        } else {
            "Esc:Close | t:Toggle tail/query mode | e:Edit | Enter:Query | j/k:Scroll | PgUp/PgDn:Page"
        };

        Paragraph::new(Line::from(vec![Span::styled(
            help_text,
            Style::default().fg(
                if self.state.is_editing
                    || self.state.start_calendar_open
                    || self.state.end_calendar_open
                {
                    Color::Yellow
                } else {
                    Color::DarkGray
                },
            ),
        )]))
        .render(area, buf);
    }

    fn render_calendar_popup(&self, area: Rect, buf: &mut Buffer) {
        // Create a centered popup for the calendar
        let popup_area = centered_rect(60, 60, area);

        // Clear background
        Clear.render(popup_area, buf);

        let title = if self.state.start_calendar_open {
            " Select Start Date "
        } else {
            " Select End Date "
        };

        let block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner_area = block.inner(popup_area);
        block.render(popup_area, buf);

        // Render a simple calendar view using text
        let selected_date = self.state.selected_date;
        let year = selected_date.year();
        let month = selected_date.month();
        let day = selected_date.day();

        let mut lines = vec![];

        // Month/Year header
        lines.push(Line::from(vec![Span::styled(
            format!("{:?} {}", month, year),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));

        // Weekday headers
        lines.push(Line::from(vec![Span::styled(
            "Su Mo Tu We Th Fr Sa",
            Style::default().fg(Color::Yellow),
        )]));

        // Get first day of month and total days
        let first_of_month = time::Date::from_calendar_date(year, month, 1).unwrap();
        let first_weekday = first_of_month.weekday().number_days_from_sunday();
        let days_in_month = time::util::days_in_year_month(year, month);

        // Build calendar grid
        let mut current_line = vec![];

        // Add leading spaces
        for _ in 0..first_weekday {
            current_line.push(Span::raw("   "));
        }

        // Add days
        for d in 1..=days_in_month {
            let style = if d == day {
                // Highlight selected day
                Style::default()
                    .fg(Color::Yellow)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else if d == time::OffsetDateTime::now_utc().day()
                && month == time::OffsetDateTime::now_utc().month()
                && year == time::OffsetDateTime::now_utc().year()
            {
                // Highlight today
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };

            current_line.push(Span::styled(format!("{:>2} ", d), style));

            // Start new line on Sunday
            if (first_weekday + d as u8) % 7 == 0 {
                lines.push(Line::from(current_line.clone()));
                current_line.clear();
            }
        }

        // Add remaining line if not empty
        if !current_line.is_empty() {
            lines.push(Line::from(current_line));
        }

        Paragraph::new(lines).render(inner_area, buf);
    }
}

/// Helper function to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
