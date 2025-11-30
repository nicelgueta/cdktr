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
use regex::Regex;

pub struct Sidebar {
    pub workflows: Vec<Workflow>,
    pub selected_index: Option<usize>,
    pub is_focused: bool,
    pub is_loading: bool,
    pub filter_input: String,
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
            filter_input: workflows_state.workflows_filter.clone(),
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.is_focused {
            Color::Cyan
        } else {
            Color::White
        };

        // Apply regex filter
        let filtered_workflows: Vec<&Workflow> = if self.filter_input.is_empty() {
            self.workflows.iter().collect()
        } else {
            match Regex::new(&self.filter_input) {
                Ok(regex) => self
                    .workflows
                    .iter()
                    .filter(|wf| regex.is_match(wf.id()) || regex.is_match(wf.name()))
                    .collect(),
                Err(_) => self.workflows.iter().collect(), // Invalid regex, show all
            }
        };

        let title = if self.is_loading {
            " Workflows (Loading...) ".to_string()
        } else if self.filter_input.is_empty() {
            format!(
                " Workflows ({}/{}) [Filter: _] ",
                filtered_workflows.len(),
                self.workflows.len()
            )
        } else {
            format!(
                " Workflows ({}/{}) [Filter: {}] ",
                filtered_workflows.len(),
                self.workflows.len(),
                self.filter_input
            )
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color));

        if filtered_workflows.is_empty() {
            block.render(area, buf);
            return;
        }

        let items: Vec<Line> = filtered_workflows
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
