use crate::dashboard::Dashboard;
use crate::control_panel::ControlPanel;
use ratatui::{crossterm::event::KeyEvent, widgets::Widget};

pub struct AppConfig {
    pub tabs: Vec<Box<dyn Component>>,
}

impl AppConfig {
    pub fn new() -> Self {
        Self {
            tabs: vec![
                Box::new(Dashboard::default()),
                Box::new(ControlPanel::default()),
            ],
        }
    }
}

pub trait Component: Widget {
    fn name(&self) -> &'static str;
    fn get_control_labels(&self) -> Vec<(&'static str, &'static str)>;
    fn handle_key_event(&mut self, ke: KeyEvent);
}
