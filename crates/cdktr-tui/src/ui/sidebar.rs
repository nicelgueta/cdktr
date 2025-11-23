/// Sidebar panel for displaying the workflow list
use crate::actions::PanelId;
use crate::stores::ui_store::UIState;
use crate::stores::workflows_store::WorkflowsState;
use cdktr_workflow::Workflow;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListState, StatefulWidget, Widget},
};

pub struct Sidebar {
    pub workflows: Vec<Workflow>,
    pub selected_index: Option<usize>,
    pub is_focused: bool,
    pub is_loading: bool,
}

impl Sidebar {
    pub fn from_state(
        workflows_state: &WorkflowsState,
        ui_state: &UIState,
        selected_index: Option<usize>,
    ) -> Self {
        Self {
            workflows: workflows_state.workflows.clone(),
            selected_index,
            is_focused: ui_state.focused_panel == PanelId::Sidebar,
            is_loading: workflows_state.is_loading,
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.is_focused {
            Color::Cyan
        } else {
            Color::White
        };

        let title = if self.is_loading {
            " Workflows (Loading...) "
        } else {
            " Workflows "
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color));

        if self.workflows.is_empty() {
            block.render(area, buf);
            return;
        }

        let items: Vec<Line> = self
            .workflows
            .iter()
            .map(|wf| Line::from(format!(" {} - {}", wf.id(), wf.name())))
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        if let Some(idx) = self.selected_index {
            list_state.select(Some(idx));
        }

        StatefulWidget::render(list, area, buf, &mut list_state);
    }
}
