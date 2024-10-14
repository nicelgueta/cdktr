use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Tabs, Widget},
};

pub struct Dashboard;

impl Dashboard {
    pub fn new() -> Self {
        Self
    }
}
impl Widget for Dashboard {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        Paragraph::new("This is the dashboard").render(area, buf);
    }
}
