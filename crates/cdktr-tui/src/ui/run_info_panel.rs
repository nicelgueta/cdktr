/// Detail panel for displaying step metadata and logs
use crate::actions::PanelId;
use crate::stores::ui_store::UIState;
use cdktr_workflow::Workflow;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Widget},
};

pub struct RunInfoPanel {
    pub selected_workflow: Option<Workflow>,
    pub is_focused: bool,
}

impl RunInfoPanel {
    pub fn new(selected_workflow: Option<Workflow>, ui_state: &UIState) -> Self {
        Self {
            selected_workflow,
            is_focused: ui_state.focused_panel == PanelId::RunInfoPanel,
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.is_focused {
            Color::Cyan
        } else {
            Color::White
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Run Info ")
            .border_style(Style::default().fg(border_color));

        if self.selected_workflow.is_some() {
            let lines = vec![
                Line::from(""),
                Line::styled("Run info to go here", Style::default().fg(Color::DarkGray)),
            ];

            Paragraph::new(lines).block(block).render(area, buf);
        } else {
            Paragraph::new("Select a workflow to view details")
                .block(block)
                .style(Style::default().fg(Color::DarkGray))
                .render(area, buf);
        }
    }
}
