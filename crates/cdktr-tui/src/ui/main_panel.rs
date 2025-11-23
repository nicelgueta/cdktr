/// Main panel for displaying workflow details and steps
use crate::actions::{PanelId, WorkflowMetadata};
use crate::stores::ui_store::UIState;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

pub struct MainPanel {
    pub selected_workflow: Option<WorkflowMetadata>,
    pub is_focused: bool,
}

impl MainPanel {
    pub fn new(selected_workflow: Option<WorkflowMetadata>, ui_state: &UIState) -> Self {
        Self {
            selected_workflow,
            is_focused: ui_state.focused_panel == PanelId::MainPanel,
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
            .title(" Workflow Details ")
            .border_style(Style::default().fg(border_color));

        if let Some(workflow) = &self.selected_workflow {
            let lines = vec![
                Line::from(vec![
                    Span::styled("ID: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&workflow.task_id),
                ]),
                Line::from(vec![
                    Span::styled("Name: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&workflow.name),
                ]),
                Line::from(vec![
                    Span::styled("Description: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&workflow.description),
                ]),
                Line::from(vec![
                    Span::styled("Path: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&workflow.path),
                ]),
                Line::from(""),
                Line::from(""),
                Line::styled(
                    "Workflow steps/DAG visualization will be implemented here",
                    Style::default().fg(Color::DarkGray),
                ),
            ];

            Paragraph::new(lines).block(block).render(area, buf);
        } else {
            Paragraph::new("No workflow selected")
                .block(block)
                .style(Style::default().fg(Color::DarkGray))
                .render(area, buf);
        }
    }
}
