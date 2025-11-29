/// Log viewer modal overlay for viewing workflow logs
use crate::common::calendar::render_calendar_below;
use crate::stores::log_viewer_store::{InputField, LogViewerState, LogViewerStore};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

pub struct LogViewerModal<'a> {
    state: LogViewerState,
    store: &'a LogViewerStore,
    // Track input field areas for calendar positioning
    start_time_area: Option<Rect>,
    end_time_area: Option<Rect>,
}

impl<'a> LogViewerModal<'a> {
    pub fn new(state: LogViewerState, store: &'a LogViewerStore) -> Self {
        Self {
            state,
            store,
            start_time_area: None,
            end_time_area: None,
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
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

    fn render_query_filters(&mut self, area: Rect, buf: &mut Buffer) {
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

        // Store areas for calendar positioning
        self.start_time_area = Some(filter_chunks[0]);
        self.end_time_area = Some(filter_chunks[1]);

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

    #[allow(dead_code)]
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

    fn render_calendar_popup(&self, screen_area: Rect, buf: &mut Buffer) {
        let (title, below_area) = if self.state.start_calendar_open {
            (" Select Start Date ", self.start_time_area)
        } else {
            (" Select End Date ", self.end_time_area)
        };

        if let Some(area) = below_area {
            render_calendar_below(screen_area, area, buf, self.state.selected_date, title);
        }
    }
}
