use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, List, ListState, Paragraph, StatefulWidget, Tabs, Widget},
};

use crate::config::Component;

const PANELS: [&'static str; 3] = ["Actions", "Agents", "Flows"];
const ACTIONS: [&'static str; 2] = ["Ping", "List Tasks"];

#[derive(Debug, Default, Clone)]
pub struct ControlPanel {
    action_state: ListState,
    panel_focussed: usize,
}

impl Component for ControlPanel {
    fn name(&self) -> &'static str {
        "Control Panel"
    }
    fn get_control_labels(&self) -> Vec<(&'static str, &'static str)> {
        vec![("<↓↑>", "Select"), ("<TAB>", "Change focus")]
    }
    fn handle_key_event(&mut self, ke: KeyEvent) {
        match ke.code {
            KeyCode::Up => self.select_action(false),
            KeyCode::Down => self.select_action(true),
            KeyCode::Tab => self.change_panel(),
            _ => (),
        }
    }
}

impl ControlPanel {
    pub fn new() -> Self {
        Self {
            action_state: ListState::default(),
            panel_focussed: 0,
        }
    }

    fn get_actions_section(&self) -> impl StatefulWidget<State = ListState> {
        List::new(ACTIONS)
            .highlight_style(Style::default().bg(Color::Cyan))
            .highlight_symbol(">")
            .block(Block::bordered().title(" Actions "))
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
    fn change_panel(&mut self) {
        if self.panel_focussed == PANELS.len() - 1 {
            self.panel_focussed = 0
        } else {
            self.panel_focussed += 1
        }
    }
    fn get_agents_section(&self) -> impl Widget {
        Paragraph::new("space").block(Block::bordered().title(" Agents "))
    }
    fn get_flows_section(&self) -> impl Widget {
        Paragraph::new("space").block(Block::bordered().title(" Flows "))
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn render() {
    //     let app = App::default();
    //     let mut buf = Buffer::empty(Rect::new(0, 0, 50, 4));

    //     app.render(buf.area, &mut buf);

    //     let mut expected = Buffer::with_lines(vec![
    //         "┏━━━━━━━━━━━━━ Counter App Tutorial ━━━━━━━━━━━━━┓",
    //         "┃                    Value: 0                    ┃",
    //         "┃                                                ┃",
    //         "┗━ Decrement <Left> Increment <Right> Quit <Q> ━━┛",
    //     ]);
    //     let title_style = Style::new().bold();
    //     let counter_style = Style::new().yellow();
    //     let key_style = Style::new().blue().bold();
    //     expected.set_style(Rect::new(14, 0, 22, 1), title_style);
    //     expected.set_style(Rect::new(28, 1, 1, 1), counter_style);
    //     expected.set_style(Rect::new(13, 3, 6, 1), key_style);
    //     expected.set_style(Rect::new(30, 3, 7, 1), key_style);
    //     expected.set_style(Rect::new(43, 3, 4, 1), key_style);

    //     // note ratatui also has an assert_buffer_eq! macro that can be used to
    //     // compare buffers and display the differences in a more readable way
    //     assert_eq!(buf, expected);
    // }

    // #[test]
    // fn handle_key_event() -> io::Result<()> {
    //     let mut app = App::default();
    //     app.handle_key_event(KeyCode::Right.into());
    //     assert_eq!(app.counter, 1);

    //     app.handle_key_event(KeyCode::Left.into());
    //     assert_eq!(app.counter, 0);

    //     let mut app = App::default();
    //     app.handle_key_event(KeyCode::Char('q').into());
    //     assert_eq!(app.exit, true);

    //     Ok(())
    // }
}
