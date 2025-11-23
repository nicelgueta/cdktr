/// Main panel for displaying workflow details and steps
use crate::actions::PanelId;
use crate::stores::ui_store::UIState;
use crate::ui::dag_viz;
use cdktr_workflow::Workflow;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

pub struct MainPanel {
    pub selected_workflow: Option<Workflow>,
    pub is_focused: bool,
}

impl MainPanel {
    pub fn new(selected_workflow: Option<Workflow>, ui_state: &UIState) -> Self {
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
            // Split the panel into two sections: details (top) and DAG (bottom)
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(6), // Details section
                    Constraint::Min(1),    // DAG section
                ])
                .split(area);

            // Render the border
            block.render(area, buf);

            // Render details section
            self.render_details(workflow, chunks[0], buf);

            // Render DAG section
            self.render_dag(workflow, chunks[1], buf);
        } else {
            Paragraph::new("No workflow selected")
                .block(block)
                .style(Style::default().fg(Color::DarkGray))
                .render(area, buf);
        }
    }

    fn render_details(&self, workflow: &Workflow, area: Rect, buf: &mut Buffer) {
        let description = workflow
            .description()
            .map(|d| d.as_str())
            .unwrap_or("No description");

        let lines = vec![
            Line::from(vec![
                Span::styled("ID: ", Style::default().fg(Color::Yellow)),
                Span::raw(workflow.id()),
            ]),
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::Yellow)),
                Span::raw(workflow.name()),
            ]),
            Line::from(vec![
                Span::styled("Description: ", Style::default().fg(Color::Yellow)),
                Span::raw(description),
            ]),
            Line::from(vec![
                Span::styled("Path: ", Style::default().fg(Color::Yellow)),
                Span::raw(workflow.path()),
            ]),
        ];

        Paragraph::new(lines).render(area, buf);
    }

    fn render_dag(&self, workflow: &Workflow, area: Rect, buf: &mut Buffer) {
        let dag = workflow.get_dag();

        // Render task summary
        let mut all_lines = dag_viz::render_task_summary(dag);
        all_lines.push(Line::from(""));

        // Render DAG tree
        all_lines.extend(dag_viz::render_dag(dag));

        Paragraph::new(all_lines).render(area, buf);
    }
}
