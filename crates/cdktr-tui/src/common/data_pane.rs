/// A Ratatui example that demonstrates how to create an interactive table with a scrollbar.
///
/// This example runs with the Ratatui library code in the branch that you are currently
/// reading. See the [`latest`] branch for the code which works with the most recent Ratatui
/// release.
///
/// [`latest`]: https://github.com/ratatui/ratatui/tree/latest
use color_eyre::Result;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{self, Color, Modifier, Style, Stylize};
use ratatui::text::Text;
use ratatui::widgets::{
    Block, BorderType, Cell, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, StatefulWidget, Table, TableState, Widget,
};
use ratatui::{DefaultTerminal, Frame};
use style::palette::tailwind;
use unicode_width::UnicodeWidthStr;

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];
const INFO_TEXT: [&str; 2] = [
    "(↑) up | (↓) down | (←/→) left/right | (Sht ←/→) toggle colors",
    "",
];

const ITEM_HEIGHT: usize = 4;

pub trait TableRow<const N: usize>
where
    Self: Sized,
{
    fn ref_array(&self) -> [&String; N];
    fn column_headers() -> [&'static str; N];
}

#[derive(Debug, Clone)]
struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,

    selected_row_style_fg: Color,
    selected_column_style_fg: Color,
    selected_cell_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    footer_border_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_row_style_fg: color.c400,
            selected_column_style_fg: color.c400,
            selected_cell_style_fg: color.c600,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}
#[derive(Debug, Clone)]
pub struct DataTable<const N: usize, T: TableRow<{ N }>> {
    state: TableState,
    items: Vec<T>,
    scroll_state: ScrollbarState,
    colors: TableColors,
    color_index: usize,
}

impl<const N: usize, T: TableRow<N>> DataTable<N, T> {
    pub fn new(data_vec: Vec<T>) -> Self {
        Self {
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new((data_vec.len() - 1) * ITEM_HEIGHT),
            colors: TableColors::new(&PALETTES[0]),
            color_index: 0,
            items: data_vec,
        }
    }

    pub fn handle_key_event(&mut self, ke: KeyEvent) {
        let shift_pressed = ke.modifiers.contains(KeyModifiers::SHIFT);
        match ke.code {
            KeyCode::Char('j') | KeyCode::Down => self.next_row(),
            KeyCode::Char('k') | KeyCode::Up => self.previous_row(),
            KeyCode::Char('l') | KeyCode::Right if shift_pressed => self.next_color(),
            KeyCode::Char('h') | KeyCode::Left if shift_pressed => {
                self.previous_color();
            }
            KeyCode::Char('l') | KeyCode::Right => self.next_column(),
            KeyCode::Char('h') | KeyCode::Left => self.previous_column(),
            _ => {}
        }
    }
    pub fn next_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn previous_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn next_column(&mut self) {
        if self.state.selected_column().unwrap_or(0) < N - 1 {
            self.state.select_next_column();
        }
    }

    pub fn previous_column(&mut self) {
        self.state.select_previous_column();
    }

    pub const fn next_color(&mut self) {
        self.color_index = (self.color_index + 1) % PALETTES.len();
    }

    pub const fn previous_color(&mut self) {
        let count = PALETTES.len();
        self.color_index = (self.color_index + count - 1) % count;
    }

    pub const fn set_colors(&mut self) {
        self.colors = TableColors::new(&PALETTES[self.color_index]);
    }

    fn render_table(&mut self, area: Rect, buf: &mut Buffer) {
        let header_style = Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg);
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg);
        let selected_col_style = Style::default().fg(self.colors.selected_column_style_fg);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_cell_style_fg);

        let header = T::column_headers()
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);
        let rows = self.items.iter().enumerate().map(|(i, data)| {
            let color = match i % 2 {
                0 => self.colors.normal_row_color,
                _ => self.colors.alt_row_color,
            };
            let item = data.ref_array();
            item.into_iter()
                .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
                .collect::<Row>()
                .style(Style::new().fg(self.colors.row_fg).bg(color))
                .height(2)
        });
        let bar = " █ ";
        StatefulWidget::render(
            Table::new(
                rows,
                constraint_len_calculator(&self.items), // [
                                                        //     // + 1 is for padding.
                                                        //     Constraint::Length(self.longest_item_lens.0 + 1),
                                                        //     Constraint::Min(self.longest_item_lens.1 + 1),
                                                        //     Constraint::Min(self.longest_item_lens.2),
                                                        // ],
            )
            .header(header)
            .row_highlight_style(selected_row_style)
            .column_highlight_style(selected_col_style)
            .cell_highlight_style(selected_cell_style)
            .highlight_symbol(Text::from(vec![
                "".into(),
                bar.into(),
                bar.into(),
                "".into(),
            ]))
            .bg(self.colors.buffer_bg)
            .highlight_spacing(HighlightSpacing::Always)
            .block(Block::bordered().title(" Workflows ")),
            area,
            buf,
            &mut self.state,
        )
    }

    fn render_scrollbar(&mut self, area: Rect, buf: &mut Buffer) {
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .render(
                area.inner(Margin {
                    vertical: 1,
                    horizontal: 1,
                }),
                buf,
                &mut self.scroll_state,
            );
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(Text::from_iter(INFO_TEXT))
            .style(
                Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
            )
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(self.colors.footer_border_color)),
            )
            .render(area, buf);
    }
}

impl<const N: usize, T: TableRow<N>> Widget for DataTable<N, T> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        let vertical = &Layout::vertical([Constraint::Min(5), Constraint::Length(3)]);
        let rects = vertical.split(area);

        self.set_colors();

        self.render_table(rects[0], buf);
        self.render_scrollbar(rects[0], buf);
        self.render_footer(rects[1], buf);
    }
}

fn constraint_len_calculator<const N: usize, T: TableRow<N>>(items: &[T]) -> [u16; N] {
    // Initialize array with zeros
    let mut max_widths = [0u16; N];

    // First get column headers width
    for (i, header) in T::column_headers().iter().enumerate() {
        max_widths[i] = header.width() as u16;
    }

    // Then check all row values
    for item in items {
        for (i, value) in item.ref_array().iter().enumerate() {
            let value_width = value.width() as u16;
            // Update max width if current value is wider
            max_widths[i] = max_widths[i].max(value_width + 1); // +1 for padding
        }
    }
    max_widths[N - 2] -= 1; // no padding on last
    max_widths
}
