use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Paragraph, Widget},
};
use std::env;

use crate::utils::center;
use cdktr_core::{
    get_server_tcp_uri, ClientResponseMessage, PrincipalAPI, API, CDKTR_DEFAULT_TIMEOUT,
};

pub const ACTIONS: [&'static str; 2] = ["Ping", "List Tasks"];

pub trait ActionPane {
    /// Format the message to be sent to the server using the struct implementing this trait
    /// and return the PrincipalAPI enum variant to be used to send the message.
    fn format_msg(&self) -> PrincipalAPI;

    /// Send the message to the server and return the response.
    async fn send_msg(&mut self) -> ClientResponseMessage {
        let msg = self.format_msg();
        let cdkr_principal_host = env::var("CDKR_PRINCIPAL_HOST").unwrap_or("0.0.0.0".to_string());
        let cdkr_principal_port = env::var("CDKR_PRINCIPAL_PORT");
        let cdkr_principal_port = match cdkr_principal_port {
            Ok(port) => port,
            Err(_) => {
                return ClientResponseMessage::ServerError(
                    "Environment variable CDKR_PRINCIPAL_PORT not set".to_string(),
                )
            }
        };
        let cdkr_principal_port = cdkr_principal_port.parse::<usize>();
        let cdkr_principal_port = match cdkr_principal_port {
            Ok(port) => port,
            Err(_) => {
                return ClientResponseMessage::ServerError(
                    "CDKR_PRINCIPAL_PORT is not a valid port number".to_string(),
                )
            }
        };
        let uri = get_server_tcp_uri(&cdkr_principal_host, cdkr_principal_port);
        let result = msg.send(&uri, CDKTR_DEFAULT_TIMEOUT).await;
        match result {
            Ok(response) => response,
            Err(e) => ClientResponseMessage::ServerError(e.to_string()),
        }
    }
}

struct FactoryConfig {
    action_upper: &'static str,
    title: &'static str,
    content: &'static str,
    // action: PrincipalAPI
}

#[derive(Debug, Clone)]
pub struct Ping;

impl Ping {
    fn new() -> Self {
        Self {}
    }
}

impl ActionPane for Ping {
    fn format_msg(&self) -> PrincipalAPI {
        PrincipalAPI::Ping
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
pub enum ActionPaneFactory {
    Ping(Ping),
    ListTasks(ListTasks),
}

impl ActionPaneFactory {
    pub fn from_str(s: &str) -> Self {
        match s {
            "Ping" => Self::Ping(Ping::new()),
            "List Tasks" => Self::ListTasks(ListTasks::new()),
            o => panic!("Tried to render unimplemented action pane: {}", o),
        }
    }
}

impl Widget for ActionPaneFactory {
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
