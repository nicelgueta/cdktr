use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Paragraph, Widget},
};

use crate::{config, utils::center};

pub const ACTIONS: [&'static str; 2] = ["Ping", "List Tasks"];

struct FactoryConfig {
    action_upper: &'static str,
    title: &'static str,
    content: &'static str,
}

#[derive(Debug, Clone)]
pub struct Ping;

impl Ping {
    fn new() -> Self {
        Self {}
    }
}

impl Widget for Ping {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let config = FactoryConfig {
            action_upper: "PING",
            title: "Ping",
            content: "ping content",
        };
        pane_factory_render(area, buf, config)
    }
}

#[derive(Debug, Clone)]
pub struct ListTasks;

impl ListTasks {
    fn new() -> Self {
        Self {}
    }
}

impl Widget for ListTasks {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let config = FactoryConfig {
            action_upper: "LISTTASKS",
            title: "List scheduled tasks",
            content: "list tasks content",
        };
        pane_factory_render(area, buf, config)
    }
}

#[derive(Debug, Clone)]
pub enum ActionPane {
    Ping(Ping),
    ListTasks(ListTasks),
}

impl ActionPane {
    pub fn from_str(s: &str) -> Self {
        match s {
            "Ping" => Self::Ping(Ping::new()),
            "List Tasks" => Self::ListTasks(ListTasks::new()),
            o => panic!("Tried to render unimplemented action pane: {}", o),
        }
    }
}

impl Widget for ActionPane {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        match self {
            Self::Ping(action_widget) => action_widget.render(area, buf),
            Self::ListTasks(action_widget) => action_widget.render(area, buf),
        }
    }
}

/// factory function used to create the action panes from configurations passed to them
/// from each action widget.
fn pane_factory_render(area: Rect, buf: &mut Buffer, config: FactoryConfig) {
    // render a border around the whole area
    Paragraph::new("")
        .block(
            Block::bordered()
                .title(format!(" {} ", config.title))
                // .border_type(ratatui::widgets::BorderType::Double)
                .border_style(Style::default().bold().fg(Color::Green)),
        )
        .render(area, buf);
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);
    Paragraph::new(config.content)
        .block(
            Block::bordered()
                .title(format!(" {} ", config.action_upper))
                .border_style(Style::default().bold().fg(Color::Cyan)),
        )
        .render(
            center(
                layout[0],
                Constraint::Percentage(90),
                Constraint::Percentage(70),
            ),
            buf,
        );

    Paragraph::new(config.content)
        .block(
            Block::bordered()
                .title(format!(" Response "))
                .border_style(Style::default().bold().fg(Color::White)),
        )
        .render(
            center(
                layout[1],
                Constraint::Percentage(90),
                Constraint::Percentage(90),
            ),
            buf,
        );
}
