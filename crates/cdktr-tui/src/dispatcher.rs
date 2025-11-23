/// Central dispatcher for the flux architecture.
/// Receives Actions and forwards them to Stores and Effects.
use crate::actions::Action;
use tokio::sync::mpsc;

/// The Dispatcher is responsible for routing Actions to all registered handlers
#[derive(Clone)]
pub struct Dispatcher {
    /// Channel sender for dispatching actions
    tx: mpsc::UnboundedSender<Action>,
}

impl Dispatcher {
    /// Create a new Dispatcher with a receiver for processing actions
    pub fn new() -> (Self, mpsc::UnboundedReceiver<Action>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { tx }, rx)
    }

    /// Dispatch an action through the system
    /// This is the main entry point for all state changes
    pub fn dispatch(&self, action: Action) {
        if let Err(e) = self.tx.send(action.clone()) {
            log::error!("Failed to dispatch action {:?}: {}", action, e);
        }
    }
}
/// ActionReceiver processes actions and routes them to stores and effects
pub struct ActionReceiver {
    rx: mpsc::UnboundedReceiver<Action>,
}

impl ActionReceiver {
    pub fn new(rx: mpsc::UnboundedReceiver<Action>) -> Self {
        Self { rx }
    }

    /// Receive the next action (blocking until one is available)
    pub async fn recv(&mut self) -> Option<Action> {
        self.rx.recv().await
    }
}
