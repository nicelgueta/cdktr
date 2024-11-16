use crate::utils::center;
use cdktr_core::{
    get_server_tcp_uri, ClientResponseMessage, PrincipalAPI, API, CDKTR_DEFAULT_TIMEOUT,
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Paragraph, Widget},
};
use std::env;

pub struct RenderConfig {
    title: &'static str,
    description: &'static str,
    msg: String,
    resp: String,
}

pub trait APIAction {
    /// Format the message to be sent to the server using the struct implementing this trait
    /// and return the PrincipalAPI enum variant to be used to send the message.
    fn format_msg(&self) -> Result<PrincipalAPI, String>;

    /// Get the factory configuration for the action pane.
    /// This is used to render the action pane.
    fn get_render_config(&self) -> RenderConfig;

    /// Send the message to the server and return the response as
    /// a raw string
    async fn send_msg(&mut self) -> String {
        let msg = match self.format_msg() {
            Ok(msg) => msg,
            Err(e) => return e,
        };
        let cdkr_principal_host = env::var("CDKTR_PRINCIPAL_HOST").unwrap_or("0.0.0.0".to_string());
        let cdkr_principal_port = env::var("CDKTR_PRINCIPAL_PORT");
        let cdkr_principal_port = match cdkr_principal_port {
            Ok(port) => port,
            Err(_) => {
                return ClientResponseMessage::ServerError(
                    "Environment variable CDKTR_PRINCIPAL_PORT not set".to_string(),
                )
                .into()
            }
        };
        let cdkr_principal_port = cdkr_principal_port.parse::<usize>();
        let cdkr_principal_port = match cdkr_principal_port {
            Ok(port) => port,
            Err(_) => {
                return ClientResponseMessage::ServerError(
                    "CDKTR_PRINCIPAL_PORT is not a valid port number".to_string(),
                )
                .into()
            }
        };
        let uri = get_server_tcp_uri(&cdkr_principal_host, cdkr_principal_port);
        let result = msg.send(&uri, CDKTR_DEFAULT_TIMEOUT).await;
        match result {
            Ok(response) => response.into(),
            Err(e) => ClientResponseMessage::NetworkError(e.to_string()).into(),
        }
    }
}

/// macro to create the action panes.
/// This is required to create each struct that will be used to render the action panes.
macro_rules! create_action {
    ($title:expr, $api_variant:ident, $desc:expr) => {
        #[derive(Debug, Clone)]
        pub struct $api_variant {
            resp: String,
            msg: String,
        }

        impl $api_variant {
            fn new() -> Self {
                Self {
                    resp: "".to_string(),
                    msg: $title.to_string(),
                }
            }
        }

        impl APIAction for $api_variant {
            fn format_msg(&self) -> Result<PrincipalAPI, String> {
                let parse_result = PrincipalAPI::try_from($title.to_string());
                match parse_result {
                    Ok(api) => Ok(api),
                    Err(e) => Err(e.to_string()),
                }
            }

            fn get_render_config(&self) -> RenderConfig {
                RenderConfig {
                    title: $title,
                    description: $desc,
                    msg: self.msg.clone(),
                    resp: self.resp.clone(),
                }
            }
        }

        impl Widget for $api_variant {
            fn render(self, area: Rect, buf: &mut Buffer)
            where
                Self: Sized,
            {
                let config = self.get_render_config();
                pane_factory_render(area, buf, config)
            }
        }
    };
}

/// macro to create the action panes.
/// This is required to create the action panes and the action handler enum
/// that will be used to render each pane. This is needed because the Widget trait
/// requires a Sized type to be passed to it. This means that simply using Box<dyn Widget>
/// will not work as the Widget trait is not object safe. So instead, we need to create
/// each action pane as a separate struct and then create an enum that will be used to
/// render its variant by also implmenting the Widget trait. As this is a lot of boilerplate
/// code and potentially error prone in ensuring that the enum and the struct are in sync,
/// this macro is used to automate the process.
macro_rules! create_actions {
    ($($title:expr, $api_variant:ident, $desc:expr);+ $(;)?) => {
        // note to self: the "$(;)?" is used to allow the macro to accept a trailing semicolon
        $(
            create_action!($title, $api_variant, $desc);
        )+

        #[derive(Debug, Clone)]
        pub enum ActionHandler {
            $($api_variant($api_variant),)+
        }

        impl Default for ActionHandler {
            fn default() -> Self {
                Self::from_str("PING")
            }
        }

        impl ActionHandler {
            pub fn from_str(s: &str) -> Self {
                match s {
                    $($title => Self::$api_variant($api_variant::new()),)+
                    o => panic!("Tried to render unimplemented action pane: {}", o),
                }
            }
            pub async fn act(&mut self) -> String {
                match self {
                    $(Self::$api_variant(action_widget) => action_widget.send_msg().await,)+
                }
            }
            pub fn update_resp(&mut self, resp: String) {
                match self {
                    $(Self::$api_variant(action_widget) => action_widget.resp = resp,)+
                }
            }
            // pub fn update_msg(&mut self, msg: String) {
            //     match self {
            //         $(Self::$api_variant(action_widget) => action_widget.msg = msg,)+
            //     }
            // }
        }

        impl Widget for ActionHandler {
            fn render(self, area: Rect, buf: &mut Buffer)
            where
                Self: Sized {
                match self {
                    $(Self::$api_variant(pane) => pane.render(area, buf),)+
                }
            }
        }

    };
}

pub const ACTIONS: [&'static str; 2] = ["PING", "LISTTASKS"];

create_actions!(
    "PING", Ping, "Ping the server to check if it is up";
    "LISTTASKS", ListTasks, "List all registered tasks on the server";
);

/// factory function used to create the action panes from configurations passed to them
/// from each action widget.
fn pane_factory_render(area: Rect, buf: &mut Buffer, config: RenderConfig) {
    // main layout
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Min(3),
            Constraint::Min(3),
            Constraint::Percentage(100),
        ])
        .split(area);

    // command description box
    Paragraph::new(config.description)
        .block(
            Block::bordered()
                .title(format!(" Description "))
                .border_style(Style::default().bold().fg(Color::LightGreen)),
        )
        .render(layout[0], buf);

    // command parameter box
    Paragraph::new(config.msg)
        .block(
            Block::bordered()
                .title(format!(" ZMQ Message "))
                .border_style(Style::default().bold().fg(Color::LightYellow)),
        )
        .render(layout[1], buf);

    // response box
    Paragraph::new(config.resp)
        .block(
            Block::bordered()
                .title(format!(" Response "))
                .border_style(Style::default().bold().fg(Color::LightMagenta)),
        )
        .render(layout[2], buf);
}
