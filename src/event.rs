use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::action::Action;

/// Polls for terminal events and sends them as raw Key actions.
/// The App is responsible for interpreting keys based on current mode/view.
pub struct EventHandler {
    tx: mpsc::UnboundedSender<Action>,
}

impl EventHandler {
    pub fn new(tx: mpsc::UnboundedSender<Action>) -> Self {
        Self { tx }
    }

    /// Start the event polling loop. Runs in a blocking thread.
    pub fn start(self) {
        tokio::task::spawn_blocking(move || {
            loop {
                if event::poll(Duration::from_millis(250)).unwrap_or(false) {
                    if let Ok(Event::Key(key)) = event::read() {
                        // Ctrl+C is always an immediate quit
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.code == KeyCode::Char('c')
                        {
                            if self.tx.send(Action::Quit).is_err() {
                                break;
                            }
                        } else if self.tx.send(Action::Key(key)).is_err() {
                            break;
                        }
                    }
                } else {
                    // Tick for background updates (e.g., running execution timers)
                    if self.tx.send(Action::Tick).is_err() {
                        break;
                    }
                }
            }
        });
    }
}
