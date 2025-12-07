/// Keyboard input handling and key mapping
use crate::actions::{Action, PanelId, TabId};
use crate::stores::{AppLogsStore, LogViewerStore, UIStore, WorkflowsStore};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::crossterm;

/// Handle keyboard input and return the appropriate Action
pub fn handle_key_event(
    key_event: KeyEvent,
    ui_store: &UIStore,
    workflows_store: &WorkflowsStore,
    app_logs_store: &AppLogsStore,
    log_viewer_store: &LogViewerStore,
) -> Option<Action> {
    let ui_state = ui_store.get_state();
    let log_viewer_state = log_viewer_store.get_state();

    // If log viewer is open, handle modal keys first
    if log_viewer_state.is_open {
        return handle_log_viewer_keys(key_event, log_viewer_store);
    }

    match key_event.code {
        // Global keys
        KeyCode::Char(':') => {
            // Start command mode
            ui_store.set_command_input(":".to_string());
            None
        }
        KeyCode::Char('q') if ui_state.command_input == ":" => {
            // Complete :q command
            Some(Action::Quit)
        }
        KeyCode::Esc if !ui_state.command_input.is_empty() => {
            // Cancel command mode
            ui_store.clear_command_input();
            None
        }
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
    let state = workflows_store.get_state();

    match key_event.code {
        // Refresh with Shift+R
        KeyCode::Char('R')
            if key_event
                .modifiers
                .contains(crossterm::event::KeyModifiers::SHIFT) =>
        {
            Some(Action::RefreshWorkflows)
        }
        KeyCode::Char('r')
            if key_event
                .modifiers
                .contains(crossterm::event::KeyModifiers::SHIFT) =>
        {
            Some(Action::RefreshWorkflows)
        }

        // Panel navigation (Tab only)
        KeyCode::Tab => Some(next_panel(*focused_panel)),

        // Sidebar (Workflows) - prioritize filter input when typing
        KeyCode::Backspace if *focused_panel == PanelId::Sidebar => {
            let mut filter = state.workflows_filter;
            filter.pop();
            Some(Action::UpdateWorkflowsFilter(filter))
        }
        KeyCode::Char(c) if *focused_panel == PanelId::Sidebar && !c.is_control() => {
            let mut filter = state.workflows_filter;
            filter.push(c);
            Some(Action::UpdateWorkflowsFilter(filter))
        }
        KeyCode::Esc if *focused_panel == PanelId::Sidebar => {
            // Clear filter
            Some(Action::UpdateWorkflowsFilter(String::new()))
        }
        // List navigation with arrow keys only
        KeyCode::Down if *focused_panel == PanelId::Sidebar => next_workflow(workflows_store),
        KeyCode::Up if *focused_panel == PanelId::Sidebar => previous_workflow(workflows_store),
        // Enter to open log viewer
        KeyCode::Enter if *focused_panel == PanelId::Sidebar => state
            .selected_workflow_id
            .map(|id| Action::OpenLogViewer(id)),

        // MainPanel scrolling (arrow keys only)
        KeyCode::Down if *focused_panel == PanelId::MainPanel => Some(Action::ScrollMainPanel(1)),
        KeyCode::Up if *focused_panel == PanelId::MainPanel => Some(Action::ScrollMainPanel(-1)),
        KeyCode::PageDown if *focused_panel == PanelId::MainPanel => {
            Some(Action::ScrollMainPanel(5))
        }
        KeyCode::PageUp if *focused_panel == PanelId::MainPanel => {
            Some(Action::ScrollMainPanel(-5))
        }

        // RunInfoPanel - prioritize filter input when typing
        KeyCode::Backspace if *focused_panel == PanelId::RunInfoPanel => {
            let mut filter = state.run_info_filter;
            filter.pop();
            Some(Action::UpdateRunInfoFilter(filter))
        }
        KeyCode::Char(c) if *focused_panel == PanelId::RunInfoPanel && !c.is_control() => {
            let mut filter = state.run_info_filter;
            filter.push(c);
            Some(Action::UpdateRunInfoFilter(filter))
        }
        KeyCode::Esc if *focused_panel == PanelId::RunInfoPanel => {
            // Clear filter
            Some(Action::UpdateRunInfoFilter(String::new()))
        }
        // RunInfoPanel scrolling with arrow keys only
        KeyCode::Down if *focused_panel == PanelId::RunInfoPanel => Some(Action::ScrollRunInfo(1)),
        KeyCode::Up if *focused_panel == PanelId::RunInfoPanel => Some(Action::ScrollRunInfo(-1)),
        KeyCode::PageDown if *focused_panel == PanelId::RunInfoPanel => {
            Some(Action::ScrollRunInfo(5))
        }
        KeyCode::PageUp if *focused_panel == PanelId::RunInfoPanel => {
            Some(Action::ScrollRunInfo(-5))
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
        PanelId::MainPanel => PanelId::RunInfoPanel,
        PanelId::RunInfoPanel => PanelId::Sidebar,
    };
    Action::FocusPanel(next)
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

    let workflow_id = state.workflows[next_index].id().clone();
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

    let workflow_id = state.workflows[prev_index].id().clone();
    Some(Action::SelectWorkflow(workflow_id))
}

/// Handle keys when log viewer modal is open
fn handle_log_viewer_keys(
    key_event: KeyEvent,
    log_viewer_store: &LogViewerStore,
) -> Option<Action> {
    let state = log_viewer_store.get_state();

    // If calendar is open, handle calendar navigation
    if state.start_calendar_open || state.end_calendar_open {
        match key_event.code {
            KeyCode::Esc => {
                log_viewer_store.close_calendar();
                None
            }
            KeyCode::Left => {
                log_viewer_store.calendar_prev_day();
                None
            }
            KeyCode::Right => {
                log_viewer_store.calendar_next_day();
                None
            }
            KeyCode::Up => {
                log_viewer_store.calendar_prev_week();
                None
            }
            KeyCode::Down => {
                log_viewer_store.calendar_next_week();
                None
            }
            KeyCode::PageUp => {
                log_viewer_store.calendar_prev_month();
                None
            }
            KeyCode::PageDown => {
                log_viewer_store.calendar_next_month();
                None
            }
            KeyCode::Enter => {
                log_viewer_store.calendar_select_date();
                None
            }
            _ => None,
        }
    } else if state.focused_field.is_some() {
        // If a field is focused, handle text input
        match key_event.code {
            KeyCode::Esc => {
                log_viewer_store.clear_focus();
                None
            }
            KeyCode::Tab => {
                if key_event
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::SHIFT)
                {
                    log_viewer_store.focus_prev_field();
                } else {
                    log_viewer_store.focus_next_field();
                }
                None
            }
            KeyCode::Backspace => {
                log_viewer_store.delete_input();
                None
            }
            KeyCode::Left => {
                log_viewer_store.cursor_left();
                None
            }
            KeyCode::Right => {
                log_viewer_store.cursor_right();
                None
            }
            KeyCode::Char(' ') => {
                // Open calendar for date fields only
                use crate::stores::log_viewer_store::InputField;
                match state.focused_field {
                    Some(InputField::StartTime) => {
                        log_viewer_store.open_start_calendar();
                        None
                    }
                    Some(InputField::EndTime) => {
                        log_viewer_store.open_end_calendar();
                        None
                    }
                    _ => {
                        // For non-date fields, treat space as a regular character
                        log_viewer_store.update_input(' ');
                        None
                    }
                }
            }
            KeyCode::Char(c) => {
                log_viewer_store.update_input(c);
                None
            }
            KeyCode::Enter => {
                // Parse and execute query on Enter
                match log_viewer_store.parse_time_inputs() {
                    Ok(_) => Some(Action::ExecuteLogQuery),
                    Err(e) => {
                        log::error!("Failed to parse time inputs: {}", e);
                        Some(Action::QueryLogsError(e))
                    }
                }
            }
            _ => None,
        }
    } else {
        // Normal navigation mode
        match key_event.code {
            KeyCode::Esc => Some(Action::CloseLogViewer),
            KeyCode::Char('t') | KeyCode::Char('T') => Some(Action::ToggleLogMode),
            KeyCode::Char('v') | KeyCode::Char('V') => Some(Action::ToggleVerboseLogging),
            KeyCode::Tab => {
                // Tab cycles through fields in query mode
                if !state.is_live_mode {
                    log_viewer_store.focus_next_field();
                }
                None
            }
            KeyCode::Enter => {
                // Execute query only in query mode when no field is focused
                if !state.is_live_mode {
                    match log_viewer_store.parse_time_inputs() {
                        Ok(_) => return Some(Action::ExecuteLogQuery),
                        Err(e) => {
                            log::error!("Failed to parse time inputs: {}", e);
                            return Some(Action::QueryLogsError(e));
                        }
                    }
                }
                None
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                log_viewer_store.toggle_auto_scroll();
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                log_viewer_store.scroll_up(1);
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                log_viewer_store.scroll_down(1);
                None
            }
            KeyCode::PageDown => {
                log_viewer_store.scroll_up(10);
                None
            }
            KeyCode::PageUp => {
                log_viewer_store.scroll_down(10);
                None
            }
            _ => None,
        }
    }
}
