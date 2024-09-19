use std::io;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Alignment, Constraint, Rect},
    style::{Style, Stylize},
    symbols::border,
    text::{Line, Text},
    widgets::{
        block::{Position, Title},
        Block, Paragraph, Row, Table, Tabs, Widget,
    },
    Frame,
};

mod tui;

#[derive(Debug, Default)]
pub struct App {
    tab: usize,
    tabs: Vec<String>,
    exit: bool,
}

impl App {
    pub fn new(tabs: Vec<String>) -> Self {
        Self {
            tab: 0,
            tabs,
            exit: false,
        }
    }
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut tui::Tui) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn change_tab(&mut self, up: bool) {
        if up {
            if self.tab == self.tabs.len() - 1 {
            } else {
                self.tab += 1
            }
        } else {
            if self.tab == 0 {
            } else {
                self.tab -= 1
            }
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('Q') => self.exit(),
            KeyCode::Left => self.change_tab(false),
            KeyCode::Right => self.change_tab(true),
            _ => {}
        }
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.size());
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let tabs = Tabs::new(self.tabs.iter().map(|tab_name| tab_name.as_str()))
            .block(Block::bordered().title(" CDKTR "))
            .style(Style::default().white())
            .highlight_style(Style::default().cyan())
            .select(self.tab)
            .divider("|")
            .padding(" ", " ");
        tabs.render(area, buf);
        let text = "Mary had a\nlittle lamb.";

        let table = text
            .split("\n")
            .map(|line: &str| -> Row { line.split_ascii_whitespace().collect() })
            .collect::<Table>()
            .widths([Constraint::Length(10); 3]);
        table.render(area, buf)
    }
}

fn main() -> io::Result<()> {
    let mut terminal = tui::init()?;
    let app_result =
        App::new(vec!["DASHBOARD".to_string(), "FLOW MANAGER".to_string()]).run(&mut terminal);
    tui::restore()?;
    app_result
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
