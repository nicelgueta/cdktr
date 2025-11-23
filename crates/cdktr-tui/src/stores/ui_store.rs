/// UIStore manages UI-specific state (focused panels, help visibility, etc.)
use crate::actions::{Action, PanelId, TabId};
use std::sync::{Arc, RwLock};

/// Internal state for UI
#[derive(Debug, Clone)]
pub struct UIState {
    /// Currently active tab
    pub active_tab: TabId,

    /// Currently focused panel
    pub focused_panel: PanelId,

    /// Whether help overlay is visible
    pub show_help: bool,

    /// Error message to display (if any)
    pub error_message: Option<String>,

    /// Whether the application should exit
    pub should_exit: bool,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            active_tab: TabId::Workflows,
            focused_panel: PanelId::Sidebar,
            show_help: false,
            error_message: None,
            should_exit: false,
        }
    }
}

/// Store that holds UI-related state
#[derive(Clone)]
pub struct UIStore {
    state: Arc<RwLock<UIState>>,
}

impl UIStore {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(UIState::default())),
        }
    }

    /// Get a read-only snapshot of the current state
    pub fn get_state(&self) -> UIState {
        self.state.read().unwrap().clone()
    }

    /// Reducer: handle an action and update state accordingly
    pub fn reduce(&self, action: &Action) {
        let mut state = self.state.write().unwrap();

        match action {
            Action::SwitchTab(tab_id) => {
                state.active_tab = *tab_id;
            }

            Action::FocusPanel(panel_id) => {
                state.focused_panel = *panel_id;
            }
            Action::ToggleHelp => {
                state.show_help = !state.show_help;
            }

            Action::ShowError(message) => {
                state.error_message = Some(message.clone());
            }

            Action::ClearError => {
                state.error_message = None;
            }

            Action::Quit => {
                state.should_exit = true;
            }

            _ => {
                // Ignore actions not relevant to this store
            }
        }
    }

    /// Check if the application should exit
    pub fn should_exit(&self) -> bool {
        self.state.read().unwrap().should_exit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let store = UIStore::new();
        let state = store.get_state();
        assert_eq!(state.focused_panel, PanelId::Sidebar);
        assert_eq!(state.show_help, false);
        assert_eq!(state.should_exit, false);
    }

    #[test]
    fn test_focus_panel() {
        let store = UIStore::new();
        store.reduce(&Action::FocusPanel(PanelId::MainPanel));

        let state = store.get_state();
        assert_eq!(state.focused_panel, PanelId::MainPanel);
    }

    #[test]
    fn test_toggle_help() {
        let store = UIStore::new();

        store.reduce(&Action::ToggleHelp);
        assert_eq!(store.get_state().show_help, true);

        store.reduce(&Action::ToggleHelp);
        assert_eq!(store.get_state().show_help, false);
    }

    #[test]
    fn test_quit() {
        let store = UIStore::new();
        store.reduce(&Action::Quit);

        assert_eq!(store.should_exit(), true);
    }
}
