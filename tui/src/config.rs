use crate::dashboard::Dashboard;
use crate::control_panel::ControlPanel;
use ratatui::{buffer::Buffer, crossterm::event::KeyEvent, layout::Rect, widgets::Widget};

pub struct AppConfig {
    pub tabs: Vec<PageComponent>,
}

impl AppConfig {
    pub fn new() -> Self {
        Self {
            tabs: vec![
                PageComponent::Dashboard(Dashboard::default()),
                PageComponent::ControlPanel(ControlPanel::default()),
            ],
        }
    }
}


pub trait Component: Widget {
    fn name(&self) -> &'static str;
    fn get_control_labels(&self) -> Vec<(&'static str, &'static str)>;
    fn handle_key_event(&mut self, ke: KeyEvent);
}

/// Enum to house the main components that will comprise each page
/// of the application. Implementing the Component trait allows
/// for a reasonable level of polymorphism for the main app loop
/// to handle each component in a similar way.
#[derive(Debug, Clone)]
pub enum PageComponent {
    Dashboard(Dashboard),
    ControlPanel(ControlPanel),
}

impl Widget for PageComponent {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            Self::Dashboard(dashboard) => dashboard.render(area, buf),
            Self::ControlPanel(control_panel) => control_panel.render(area, buf),
        }
    }
}

impl Component for PageComponent {
    fn name(&self) -> &'static str {
        match self {
            Self::Dashboard(dashboard) => dashboard.name(),
            Self::ControlPanel(control_panel) => control_panel.name(),
        }
    }

    fn get_control_labels(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            Self::Dashboard(dashboard) => dashboard.get_control_labels(),
            Self::ControlPanel(control_panel) => control_panel.get_control_labels(),
        }
    }

    fn handle_key_event(&mut self, ke: KeyEvent) {
        match self {
            Self::Dashboard(dashboard) => dashboard.handle_key_event(ke),
            Self::ControlPanel(control_panel) => control_panel.handle_key_event(ke),
        }
    }
}
