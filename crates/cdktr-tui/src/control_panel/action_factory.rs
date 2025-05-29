use cdktr_core::{get_cdktr_setting, zmq_helpers::get_server_tcp_uri};

use cdktr_ipc::prelude::{ClientResponseMessage, PrincipalAPI, API};
use log::warn;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Paragraph, Widget},
};
use std::{collections::HashMap, env, time::Duration};

pub struct RenderConfig<'a> {
    title: &'static str,
    description: &'static str,
    action: &'a String,
    params: &'a Vec<String>,
    param_titles: &'a HashMap<usize, String>,
    resp: &'a String,
    param_focussed: usize,
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
        let cdkr_principal_port = get_cdktr_setting!(CDKTR_PRINCIPAL_PORT, usize);
        let uri = get_server_tcp_uri(&cdkr_principal_host, cdkr_principal_port);
        let result = msg
            .send(
                &uri,
                Duration::from_millis(get_cdktr_setting!(CDKTR_DEFAULT_TIMEOUT_MS, usize) as u64),
            )
            .await;
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
            action: String,
            params: Vec<String>,
            param_titles: HashMap<usize, String>,
            param_focussed: usize,
        }

        impl $api_variant {
            fn new(params: Option<Vec<String>>) -> Self {
                let params = params.unwrap_or(Vec::new());
                let mut param_titles: HashMap<usize, String> = HashMap::new();
                let mut param_v = Vec::with_capacity(params.len());
                for (i, param) in params.iter().enumerate() {
                    param_titles.insert(i, param.clone());
                    param_v.push(String::new())
                }
                Self {
                    resp: "".to_string(),
                    action: $title.to_string(),
                    params: param_v,
                    param_titles,
                    param_focussed: 0,
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
                    action: &self.action,
                    resp: &self.resp,
                    params: &self.params,
                    param_titles: &self.param_titles,
                    param_focussed: self.param_focussed,
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
    ($($title:expr, $api_variant:ident, $desc:expr, $params:expr);+ $(;)?) => {
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
                    $($title => Self::$api_variant($api_variant::new($params)),)+
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
            pub fn toggle_param(&mut self) {
                match self {
                    $(Self::$api_variant(action_widget) => {
                        if action_widget.params.len() == 0 {
                            // pass
                        } else if action_widget.param_focussed == action_widget.params.len() - 1 {
                            action_widget.param_focussed = 0
                        } else {
                            action_widget.param_focussed += 1
                        }
                    },)+
                }
            }

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

pub const ACTIONS: [&'static str; 3] = ["PING", "LSWORKFLOWS", "CREATETASK"];

fn stv(v: Vec<&str>) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}
create_actions!(
    "PING", Ping, "Ping the server to check if it is up", None;
    "LSWORKFLOWS", ListWorkflowStore, "List all registered tasks on the server", None;
    "CREATETASK", CreateTasks, "Create a new scheduled task", Some(stv(vec!["task_name", "task_type", "command", "args", "cron"]));
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

    let mut command_constraints = vec![Constraint::Min(1)];
    let no_params = config.params.len();
    for _i in 1..no_params {
        command_constraints.push(Constraint::Min(1));
    }
    let command_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(command_constraints)
        .split(layout[1]);

    // render the action as the first item
    Paragraph::new(config.action.as_str())
        .block(
            Block::bordered()
                .title(format!(" ZMQ Msg "))
                .border_style(Style::default().bold().fg(Color::LightYellow)),
        )
        .render(command_layout[0], buf);

    // render the params
    for i in 1..no_params {
        let param = config
            .param_titles
            .get(&i)
            .expect("mismatch between param count and title hashmap");
        let mut block = Block::bordered().title(format!(" {param} "));

        block = if config.param_focussed == i {
            block.border_style(Style::default().bold().fg(Color::LightYellow))
        } else {
            block
        };
        Paragraph::new("")
            .block(block)
            .render(command_layout[i], buf);
    }
    // response box
    Paragraph::new(config.resp.clone())
        .block(
            Block::bordered()
                .title(format!(" Response "))
                .border_style(Style::default().bold().fg(Color::LightMagenta)),
        )
        .render(layout[2], buf);
}
