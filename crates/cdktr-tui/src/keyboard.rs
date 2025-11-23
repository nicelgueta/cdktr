/// Keyboard input handling and key mapping
use crate::actions::{Action, PanelId, TabId};
use crate::stores::{AppLogsStore, UIStore, WorkflowsStore};
use ratatui::crossterm::event::{KeyCode, KeyEvent};

/// Handle keyboard input and return the appropriate Action
pub fn handle_key_event(
    key_event: KeyEvent,
    ui_store: &UIStore,
    workflows_store: &WorkflowsStore,
    app_logs_store: &AppLogsStore,
) -> Option<Action> {
    let ui_state = ui_store.get_state();

    match key_event.code {
        // Global keys
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(Action::Quit),
        KeyCode::Char('?') => Some(Action::ToggleHelp),

        // Tab switching
        KeyCode::Char('1') => Some(Action::SwitchTab(TabId::Workflows)),
        KeyCode::Char('2') => Some(Action::SwitchTab(TabId::Admin)),

        // Tab-specific navigation
        _ => match ui_state.active_tab {
            TabId::Workflows => {
                handle_workflows_tab_keys(key_event, &ui_state.focused_panel, workflows_store)
            }
            TabId::Admin => handle_admin_tab_keys(key_event, app_logs_store),
        },
    }
}

fn handle_workflows_tab_keys(
    key_event: KeyEvent,
    focused_panel: &PanelId,
    workflows_store: &WorkflowsStore,
) -> Option<Action> {
    match key_event.code {
        KeyCode::Char('r') | KeyCode::Char('R') => Some(Action::RefreshWorkflows),

        // Panel navigation (h/l or Tab)
        KeyCode::Char('h') => Some(previous_panel(*focused_panel)),
        KeyCode::Char('l') => Some(next_panel(*focused_panel)),
        KeyCode::Tab => Some(next_panel(*focused_panel)),

        // List navigation (j/k or Up/Down) - only in sidebar
        KeyCode::Char('j') | KeyCode::Down if *focused_panel == PanelId::Sidebar => {
            next_workflow(workflows_store)
        }
        KeyCode::Char('k') | KeyCode::Up if *focused_panel == PanelId::Sidebar => {
            previous_workflow(workflows_store)
        }

        _ => None,
    }
}

fn handle_admin_tab_keys(key_event: KeyEvent, app_logs_store: &AppLogsStore) -> Option<Action> {
    match key_event.code {
        // Scroll logs
        KeyCode::Char('j') | KeyCode::Down => {
            app_logs_store.scroll_down(1);
            None // No action needed, store updated directly
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app_logs_store.scroll_up(1);
            None
        }
        KeyCode::PageDown => {
            app_logs_store.scroll_down(10);
            None
        }
        KeyCode::PageUp => {
            app_logs_store.scroll_up(10);
            None
        }
        KeyCode::Char('g') => {
            app_logs_store.scroll_to_bottom();
            None
        }

        _ => None,
    }
}
/// Move to the next panel
fn next_panel(current: PanelId) -> Action {
    let next = match current {
        PanelId::Sidebar => PanelId::MainPanel,
        PanelId::MainPanel => PanelId::DetailPanel,
        PanelId::DetailPanel => PanelId::Sidebar,
    };
    Action::FocusPanel(next)
}

/// Move to the previous panel
fn previous_panel(current: PanelId) -> Action {
    let prev = match current {
        PanelId::Sidebar => PanelId::DetailPanel,
        PanelId::MainPanel => PanelId::Sidebar,
        PanelId::DetailPanel => PanelId::MainPanel,
    };
    Action::FocusPanel(prev)
}

/// Select the next workflow in the list
fn next_workflow(workflows_store: &WorkflowsStore) -> Option<Action> {
    let state = workflows_store.get_state();
    if state.workflows.is_empty() {
        return None;
    }

    let current_index = workflows_store.get_selected_index().unwrap_or(0);
    let next_index = if current_index >= state.workflows.len() - 1 {
        0
    } else {
        current_index + 1
    };

    let workflow_id = state.workflows[next_index].id.clone();
    Some(Action::SelectWorkflow(workflow_id))
}

/// Select the previous workflow in the list
fn previous_workflow(workflows_store: &WorkflowsStore) -> Option<Action> {
    let state = workflows_store.get_state();
    if state.workflows.is_empty() {
        return None;
    }

    let current_index = workflows_store.get_selected_index().unwrap_or(0);
    let prev_index = if current_index == 0 {
        state.workflows.len() - 1
    } else {
        current_index - 1
    };

    let workflow_id = state.workflows[prev_index].id.clone();
    Some(Action::SelectWorkflow(workflow_id))
}
