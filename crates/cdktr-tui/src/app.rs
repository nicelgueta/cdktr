/// Main application struct and event loop
use crate::actions::Action;
use crate::dispatcher::{ActionReceiver, Dispatcher};
use crate::effects::Effects;
use crate::keyboard;
use crate::logger::LogBuffer;
use crate::stores::{AppLogsStore, LogViewerStore, LogsStore, UIStore, WorkflowsStore};
use crate::ui::render_layout;
use ratatui::crossterm::event::{self, Event, KeyEventKind, MouseEventKind};
use std::io;
use std::time::Duration;

/// The main application structure following flux architecture
pub struct App {
    /// Dispatcher for sending actions
    dispatcher: Dispatcher,

    /// Store for workflow state
    workflows_store: WorkflowsStore,

    /// Store for UI state
    ui_store: UIStore,

    /// Store for logs state
    logs_store: LogsStore,

    /// Store for application logs
    app_logs_store: AppLogsStore,

    /// Store for log viewer modal
    log_viewer_store: LogViewerStore,

    /// Log buffer for capturing application logs
    log_buffer: LogBuffer,

    /// Effects handler for side effects
    effects: Effects,
}
impl App {
    pub fn new() -> Result<(Self, ActionReceiver), Box<dyn std::error::Error>> {
        let (dispatcher, rx) = Dispatcher::new();
        let action_receiver = ActionReceiver::new(rx);

        let log_buffer = crate::logger::init_memory_logger()?;

        let workflows_store = WorkflowsStore::new();
        let ui_store = UIStore::new();
        let logs_store = LogsStore::new();
        let app_logs_store = AppLogsStore::new(log_buffer.clone());
        let log_viewer_store = LogViewerStore::new();
        let mut effects = Effects::new(dispatcher.clone());
        effects.set_log_viewer_store(log_viewer_store.clone());
        effects.set_workflows_store(workflows_store.clone());

        // Spawn background tasks for status monitoring and workflow refresh
        effects.spawn_background_tasks();

        Ok((
            Self {
                dispatcher,
                workflows_store,
                ui_store,
                logs_store,
                app_logs_store,
                log_viewer_store,
                log_buffer,
                effects,
            },
            action_receiver,
        ))
    }

    /// Main event loop
    pub async fn run(
        &mut self,
        terminal: &mut crate::tui::Tui,
        mut action_receiver: ActionReceiver,
    ) -> io::Result<()> {
        // Log startup to verify logger is working
        log::info!("CDKTR TUI started successfully");
        log::debug!("Logger initialized and capturing to memory buffer");

        // Initial load of workflows
        self.dispatcher.dispatch(Action::RefreshWorkflows);

        loop {
            // Render the UI
            terminal.draw(|frame| {
                render_layout(
                    frame,
                    &self.workflows_store,
                    &self.ui_store,
                    &self.logs_store,
                    &self.app_logs_store,
                    &self.log_viewer_store,
                );
            })?;

            // Check if we should exit
            if self.ui_store.should_exit() {
                break;
            }

            // Use tokio::select to handle both UI events and actions
            tokio::select! {
                // Poll for keyboard and mouse events
                _ = tokio::time::sleep(Duration::from_millis(16)) => {
                    if event::poll(Duration::from_millis(0))? {
                        match event::read()? {
                            Event::Key(key_event) => {
                                // Only process key press events (not release)
                                if key_event.kind == KeyEventKind::Press {
                                    if let Some(action) = keyboard::handle_key_event(
                                        key_event,
                                        &self.ui_store,
                                        &self.workflows_store,
                                        &self.app_logs_store,
                                        &self.log_viewer_store,
                                    ) {
                                        self.dispatcher.dispatch(action);
                                    }
                                }
                            }
                            Event::Mouse(mouse_event) => {
                                // Handle mouse clicks for calendar date selection
                                if mouse_event.kind == MouseEventKind::Down(ratatui::crossterm::event::MouseButton::Left) {
                                    if let Some(action) = keyboard::handle_mouse_event(
                                        mouse_event,
                                        &self.log_viewer_store,
                                    ) {
                                        self.dispatcher.dispatch(action);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }

                // Process actions from the dispatcher
                Some(action) = action_receiver.recv() => {
                    self.handle_action(&action);
                }
            }
        }

        Ok(())
    }

    /// Handle an action by routing it to stores and effects
    fn handle_action(&mut self, action: &Action) {
        // Log the action for debugging
        log::debug!("Handling action: {:?}", action);

        // Route to stores (reducers)
        self.workflows_store.reduce(action);
        self.ui_store.reduce(action);
        self.logs_store.reduce(action);
        self.app_logs_store.dispatch(action);
        self.log_viewer_store.reduce(action);

        // Trigger side effects
        self.effects.handle(action);
    }
}
