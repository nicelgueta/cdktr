use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Tabs, Widget},
};

use crate::config::Component;

#[derive(Debug, Default, Clone)]
pub struct Dashboard;

impl Component for Dashboard {
    fn name(&self) -> &'static str {
        "Dashboard"
    }
    fn get_control_labels(&self) -> Vec<(&'static str, &'static str)> {
        Vec::new()
    }
    async fn handle_key_event(&mut self, ke: ratatui::crossterm::event::KeyEvent) {}
}

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
        Paragraph::new("This is the dashboard")
            .block(Block::bordered())
            .render(area, buf);
    }
}
