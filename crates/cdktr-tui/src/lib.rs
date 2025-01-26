use core::panic;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Tabs, Widget},
    Frame,
};
use std::io;

mod config;
mod control_panel;
mod dashboard;
mod tui;
mod utils;

use config::{Component, PageComponent};

struct App {
    tab: usize,
    tabs: Vec<PageComponent>,
    exit: bool,
}

impl App {
    pub fn new(ac: config::AppConfig) -> Self {
        Self {
            tab: 1,
            tabs: ac.tabs,
            exit: false,
        }
    }
    /// runs the application's main loop until the user quits
    pub async fn run(&mut self, terminal: &mut tui::Tui) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            match self.handle_events().await {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn change_tab(&mut self, i: usize) {
        if i >= self.tabs.len() {
            // do nothing for unindexed tabs
        } else {
            self.tab = i
        }
    }

    async fn handle_key_event(&mut self, key_event: KeyEvent) {
        // check
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('Q') => self.exit(),
            KeyCode::Char('1') => self.change_tab(0),
            KeyCode::Char('2') => self.change_tab(1),
            KeyCode::Char('3') => self.change_tab(2),
            _ => {
                if self.tab >= self.tabs.len() {
                    panic!("Somehow managed to get to an out of scope tab")
                } else {
                    self.tabs[self.tab].handle_key_event(key_event).await
                }
            }
        }
    }

    async fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event).await
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
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Max(3),
                Constraint::Min(1),
                Constraint::Length(2),
            ])
            .split(area);

        // tab headers
        let header_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(100)])
            .split(vertical_chunks[0]);

        // tabs
        Tabs::new(self.tabs.iter().map(|tab_page| tab_page.name()))
            .block(Block::bordered().title(" CDKTR "))
            .style(Style::default().white())
            .highlight_style(Style::default().cyan())
            .select(self.tab)
            .divider("|")
            .padding(" ", " ")
            .render(header_chunks[0], buf);

        // content
        // Ideally not have to clone the component here but otherwise
        // it'd have to move out of shared reference for render. Maybe a
        // higher-order component that doesn't require ownership?
        let component = self.tabs[self.tab].clone();
        component.render(vertical_chunks[1], buf);
        // controls
        let mut control_line = Line::from("");
        let controls = {
            let mut base_controls = vec![("<Q>", "Quit"), ("<1..9>", "Change screen")];
            base_controls.push(("", " |"));
            let screen_controls = self.tabs[self.tab].get_control_labels();
            base_controls.extend(screen_controls.iter());
            base_controls
        };
        for (ctrl, label) in controls {
            control_line.push_span(Span::raw(label));
            control_line.push_span(Span::raw(" "));
            control_line.push_span(Span::raw(ctrl).bg(Color::White).bold().underlined());
            control_line.push_span(Span::raw(" "));
        }
        let controls_text = Text::from(control_line);
        Paragraph::new(controls_text)
            .style(Style::default().white().fg(Color::DarkGray))
            .render(vertical_chunks[2], buf);
    }
}

pub async fn tui_main() -> io::Result<()> {
    let mut terminal = tui::init()?;
    let app_config = config::AppConfig::new();
    let app_result = App::new(app_config).run(&mut terminal).await;
    tui::restore()?;
    if let Err(e) = app_result {
        println!("Error: {:?}", e);
        return Err(e);
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {

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
