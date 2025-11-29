/// Reusable calendar widget for date selection
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};
use time::{Date, OffsetDateTime};

/// Calendar widget that displays a monthly calendar view
pub struct CalendarWidget {
    /// The date to display and highlight
    pub selected_date: Date,
    /// Optional title for the calendar
    pub title: Option<String>,
    /// Whether to show a border
    pub bordered: bool,
    /// Whether to highlight today's date
    pub highlight_today: bool,
}

impl CalendarWidget {
    /// Create a new calendar widget for the given date
    pub fn new(selected_date: Date) -> Self {
        Self {
            selected_date,
            title: None,
            bordered: true,
            highlight_today: true,
        }
    }

    /// Set a title for the calendar
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set whether to show a border
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Set whether to highlight today's date
    pub fn highlight_today(mut self, highlight_today: bool) -> Self {
        self.highlight_today = highlight_today;
        self
    }

    /// Render the calendar widget
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let year = self.selected_date.year();
        let month = self.selected_date.month();
        let day = self.selected_date.day();

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
        let days_in_month = month.length(year);

        // Build calendar grid
        let mut current_line = vec![];

        // Add leading spaces
        for _ in 0..first_weekday {
            current_line.push(Span::raw("   "));
        }

        let today = if self.highlight_today {
            Some(OffsetDateTime::now_utc())
        } else {
            None
        };

        // Add days
        for d in 1..=days_in_month {
            let style = if d == day {
                // Highlight selected day
                Style::default()
                    .fg(Color::Yellow)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else if let Some(today_dt) = today {
                if d == today_dt.day() && month == today_dt.month() && year == today_dt.year() {
                    // Highlight today
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                }
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

        let paragraph = Paragraph::new(lines).alignment(Alignment::Center);

        if self.bordered {
            let title = self
                .title
                .as_ref()
                .map(|t| t.as_str())
                .unwrap_or(" Calendar ");
            let block = Block::default()
                .title(title)
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));

            let inner_area = block.inner(area);
            block.render(area, buf);
            paragraph.render(inner_area, buf);
        } else {
            paragraph.render(area, buf);
        }
    }
}

/// Render a calendar popup in the center of the given area
pub fn render_calendar_popup(area: Rect, buf: &mut Buffer, selected_date: Date, title: &str) {
    // Create a centered popup for the calendar
    let popup_area = centered_rect(60, 60, area);

    // Clear background
    Clear.render(popup_area, buf);

    // Render calendar
    CalendarWidget::new(selected_date)
        .title(title)
        .bordered(true)
        .highlight_today(true)
        .render(popup_area, buf);
}

/// Render a calendar popup positioned below a specific area (like an input field)
pub fn render_calendar_below(
    screen_area: Rect,
    below_area: Rect,
    buf: &mut Buffer,
    selected_date: Date,
    title: &str,
) {
    // Calendar needs: 1 title + 1 month/year + 1 blank + 1 weekdays + up to 6 weeks + borders
    // = 3 (top) + 7 (max weeks) + 2 (borders) = 12 lines with padding
    let calendar_height = 12;
    let calendar_width = 26; // "Su Mo Tu We Th Fr Sa" = 20 chars + 4 padding + 2 borders

    // Position below the input area, aligned to its left edge
    let x = below_area.x;
    let y = below_area.y + below_area.height;

    // Ensure calendar doesn't go off screen
    let x = x.min(screen_area.width.saturating_sub(calendar_width));
    let y = y.min(screen_area.height.saturating_sub(calendar_height));

    let popup_area = Rect {
        x,
        y,
        width: calendar_width,
        height: calendar_height,
    };

    // Clear background
    Clear.render(popup_area, buf);

    // Render calendar
    CalendarWidget::new(selected_date)
        .title(title)
        .bordered(true)
        .highlight_today(true)
        .render(popup_area, buf);
}

/// Helper function to create a centered rectangle
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
