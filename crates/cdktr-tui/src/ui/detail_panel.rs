/// Detail panel for displaying step metadata and logs
use crate::actions::{PanelId, WorkflowMetadata};
use crate::stores::ui_store::UIState;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Widget},
};

pub struct DetailPanel {
    pub selected_workflow: Option<WorkflowMetadata>,
    pub is_focused: bool,
}

impl DetailPanel {
    pub fn new(selected_workflow: Option<WorkflowMetadata>, ui_state: &UIState) -> Self {
        Self {
            selected_workflow,
            is_focused: ui_state.focused_panel == PanelId::DetailPanel,
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
            .title(" Step Details & Logs ")
            .border_style(Style::default().fg(border_color));

        if self.selected_workflow.is_some() {
            let lines = vec![
                Line::from(""),
                Line::styled(
                    "Step metadata and logs will be displayed here",
                    Style::default().fg(Color::DarkGray),
                ),
                Line::from(""),
                Line::styled(
                    "Future: Real-time log streaming",
                    Style::default().fg(Color::DarkGray),
                ),
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
