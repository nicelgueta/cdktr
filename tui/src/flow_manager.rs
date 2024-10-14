use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Tabs, Widget},
};

use crate::config::Controls;

pub struct FlowManagerControls;

impl Controls for FlowManagerControls {
    fn get() -> Vec<(&'static str, &'static str)> {
        Vec::new()
    }
}
pub struct FlowManager;

impl FlowManager {
    pub fn new() -> Self {
        Self
    }
}
impl Widget for FlowManager {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        Paragraph::new("This is the FlowManager")
            .block(Block::bordered())
            .render(area, buf);
    }
}
