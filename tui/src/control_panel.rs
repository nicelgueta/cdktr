use std::collections::HashMap;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, List, ListState, Paragraph, StatefulWidget, Widget},
};

use crate::{config::Component, utils::vec_to_hashmap};

const PANELS: [&'static str; 3] = ["Actions", "Agents", "Flows"];
const ACTIONS: [&'static str; 2] = ["Ping", "List Tasks"];

#[derive(Debug, Default, Clone)]
pub struct ControlPanel {
    action_state: ListState,
    panel_focussed: usize,
    action_modal_open: bool,
    action_panes: HashMap<&'static str, &'static str>
}

impl Component for ControlPanel {
    fn name(&self) -> &'static str {
        "Control Panel"
    }
    fn get_control_labels(&self) -> Vec<(&'static str, &'static str)> {
        let mut base_controls = vec![("<↓↑>", "Select"), ("<TAB>", "Change focus")];
        if self.action_modal_open {
            base_controls.push(("<C>", "Close"))
        };
        base_controls
    }
    fn handle_key_event(&mut self, ke: KeyEvent) {
        match ke.code {
            KeyCode::Up => self.select_action(false),
            KeyCode::Down => self.select_action(true),
            KeyCode::Tab => self.change_panel(),
            KeyCode::Enter => self.action_enter(),
            KeyCode::Char('c') => self.action_modal_open = false,
            _ => (),
        }
    }
}

impl ControlPanel {
    pub fn new() -> Self {
        let mut instance = Self {
            action_state: ListState::default(),
            panel_focussed: 0,
            action_modal_open: false,
            action_panes: vec_to_hashmap(vec![
                ("Ping", "summary"),
                ("List Tasks", "summary"),
            ]),
        };
        instance.focus_panel();
        instance
    }
    fn panel_highlighted_color(&self, tab_name: &str) -> Color {
        if PANELS[self.panel_focussed] == tab_name {
            Color::Rgb(123, 201, 227)
        } else {
            Color::White
        }
    }
    fn action_enter(&mut self) {
        match PANELS[self.panel_focussed] {
            "Actions" => {
                if self.action_modal_open {
                    // TODO: do something with the action
                } else {
                    self.action_modal_open = true
                }
            }
            _ => (),
        }
    }
    fn select_action(&mut self, next: bool) {
        if self.panel_focussed == 0 {
            let selected = self
                .action_state
                .selected()
                .expect("Should automatically have a selected item if action box is focussed");
            if next {
                if selected < ACTIONS.len() - 1 {
                    self.action_state.select_next();
                }
            } else {
                if selected > 0 {
                    self.action_state.select_previous();
                }
            }
        }
    }
    fn focus_panel(&mut self) {
        match PANELS[self.panel_focussed] {
            "Actions" => self.action_state.select_first(),
            _ => (),
        }
    }
    fn unfocus_panel(&mut self) {
        match PANELS[self.panel_focussed] {
            "Actions" => {
                self.action_state.select(None);
                self.action_modal_open = false
            }
            _ => (),
        }
    }
    fn change_panel(&mut self) {
        // unfocus current panel
        self.unfocus_panel();

        if self.panel_focussed == PANELS.len() - 1 {
            self.panel_focussed = 0
        } else {
            self.panel_focussed += 1
        }

        // focus new one
        self.focus_panel();
    }
    fn get_actions_section(&self) -> impl StatefulWidget<State = ListState> {
        List::new(ACTIONS)
            .highlight_style(Style::default().bg(Color::Cyan))
            .highlight_symbol(">")
            .block(
                Block::bordered()
                    .title(" Principal Actions ")
                    .fg(self.panel_highlighted_color("Actions")),
            )
    }
    fn get_agents_section(&self) -> impl Widget {
        Paragraph::new("space").block(
            Block::bordered()
                .title(" Agents ")
                .fg(self.panel_highlighted_color("Agents")),
        )
    }
    fn get_flows_section(&self) -> impl Widget {
        Paragraph::new("space").block(
            Block::bordered()
                .title(" Flows ")
                .fg(self.panel_highlighted_color("Flows")),
        )
    }
    fn get_action_modal(&self) -> impl Widget {
        let selected_action = ACTIONS[self.action_state.selected().expect("Failed to access selected item from action state")];
        let text = "summarty";
        Paragraph::new(text).block(
            Block::bordered()
                .title(format!(" ACTION: {selected_action} "))
                // .border_type(ratatui::widgets::BorderType::)
                .border_style(Style::default().bold().fg(Color::Green)),
        )
    }
}
impl Widget for ControlPanel {
    fn render(mut self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        // layouts
        let main_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);

        let left_sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(main_layout[0]);

        let action_section = self.get_actions_section();
        StatefulWidget::render(
            action_section,
            left_sections[0],
            buf,
            &mut self.action_state,
        );
        self.get_agents_section().render(left_sections[1], buf);
        self.get_flows_section().render(main_layout[1], buf);

        if self.action_modal_open {
            // use the flows section for the action modal to avoid an uneat popup
            // self.get_action_modal().render(center(area, Constraint::Percentage(70), Constraint::Percentage(70)), buf)
            self.get_action_modal().render(main_layout[1], buf)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // control panel tests
    //

    #[test]
    fn test_panel_highlighted_color() {
        let mut control_panel = ControlPanel::new();
        control_panel.panel_focussed = 0;
        assert_eq!(control_panel.panel_highlighted_color("Actions"), Color::Rgb(123, 201, 227));
        assert_eq!(control_panel.panel_highlighted_color("Agents"), Color::White);
        assert_eq!(control_panel.panel_highlighted_color("Flows"), Color::White);
    }

    #[test]
    fn test_select_action() {
        let mut control_panel = ControlPanel::new();
        assert_eq!(control_panel.action_state.selected(), Some(0));
        control_panel.select_action(true);
        assert_eq!(control_panel.action_state.selected(), Some(1));
        control_panel.select_action(true);
        assert_eq!(control_panel.action_state.selected(), Some(1));
        control_panel.select_action(false);
        assert_eq!(control_panel.action_state.selected(), Some(0));
        control_panel.select_action(false);
        assert_eq!(control_panel.action_state.selected(), Some(0));
    }

    #[test]
    fn test_starts_with_focussed_panel() {
        let control_panel = ControlPanel::new();
        assert_eq!(control_panel.panel_focussed, 0);
        assert_eq!(control_panel.action_state.selected(), Some(0));
    }

    #[test]
    fn test_unfocus_panel() {
        let mut control_panel = ControlPanel::new();

        control_panel.focus_panel();

        assert_eq!(control_panel.action_state.selected(), Some(0));
        control_panel.unfocus_panel();
        assert_eq!(control_panel.action_state.selected(), None);
    }

    #[test]
    fn test_change_panel() {
        let mut control_panel = ControlPanel::new();
        assert_eq!(control_panel.panel_focussed, 0);

        control_panel.change_panel();
        assert_eq!(control_panel.panel_focussed, 1);

        control_panel.change_panel();
        assert_eq!(control_panel.panel_focussed, 2);

        control_panel.change_panel();
        assert_eq!(control_panel.panel_focussed, 0);
    }

    #[test]
    fn test_get_actions_enter() {
        let mut control_panel = ControlPanel::new();
        control_panel.action_enter();
        assert_eq!(control_panel.action_modal_open, true);
        control_panel.action_enter();
        assert_eq!(control_panel.action_modal_open, true);
    }

    #[test]
    fn test_handle_key_event() {
        let mut control_panel = ControlPanel::new();
        control_panel.handle_key_event(KeyEvent::from(KeyCode::Up));
        assert_eq!(control_panel.action_state.selected(), Some(0));
        control_panel.handle_key_event(KeyEvent::from(KeyCode::Down));
        assert_eq!(control_panel.action_state.selected(), Some(1));
        control_panel.handle_key_event(KeyEvent::from(KeyCode::Down));
        assert_eq!(control_panel.action_state.selected(), Some(1));
        control_panel.handle_key_event(KeyEvent::from(KeyCode::Up));
        assert_eq!(control_panel.action_state.selected(), Some(0));
        control_panel.handle_key_event(KeyEvent::from(KeyCode::Tab));
        assert_eq!(control_panel.panel_focussed, 1);
        control_panel.handle_key_event(KeyEvent::from(KeyCode::Tab));
        assert_eq!(control_panel.panel_focussed, 2);
        control_panel.handle_key_event(KeyEvent::from(KeyCode::Tab));
        assert_eq!(control_panel.panel_focussed, 0);
        control_panel.handle_key_event(KeyEvent::from(KeyCode::Enter));
        assert_eq!(control_panel.action_modal_open, true);
        control_panel.handle_key_event(KeyEvent::from(KeyCode::Enter));
        assert_eq!(control_panel.action_modal_open, true);
        control_panel.handle_key_event(KeyEvent::from(KeyCode::Char('c')));
        assert_eq!(control_panel.action_modal_open, false);
    }
}
