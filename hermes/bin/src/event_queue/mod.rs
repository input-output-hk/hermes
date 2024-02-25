//! Hermes event queue implementation.

use std::sync::{
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};

use self::event::HermesEvent;
use crate::runtime_extensions::state::State;

pub(crate) mod event;

/// Hermes event queue error
#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum Error {
    /// Failed to add event into the event queue. Event queue is closed.
    #[error("Failed to add event into the event queue. Event queue is closed.")]
    QueueClosed,
}

/// Hermes event queue
pub(crate) struct HermesEventQueue {
    /// Hermes event queue sender
    sender: Sender<HermesEvent>,
    /// Hermes event queue receiver
    receiver: Mutex<Receiver<HermesEvent>>,
}

impl HermesEventQueue {
    /// Create a new Hermes event queue
    pub(crate) fn new() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        Self {
            sender,
            receiver: Mutex::new(receiver),
        }
    }

    /// Add event into the event queue
    #[allow(
        clippy::unnecessary_wraps,
        clippy::unused_self,
        unused_variables,
        clippy::needless_pass_by_value
    )]
    pub(crate) fn add(&self, event: HermesEvent) -> anyhow::Result<()> {
        self.sender.send(event).map_err(|_| Error::QueueClosed)?;
        Ok(())
    }

    /// Executes Hermes events from the event queue.
    ///
    /// # Note:
    /// This is a blocking call and consumes the event queue.
    #[allow(clippy::unnecessary_wraps, clippy::unwrap_used)]
    pub(crate) fn event_execution_loop(&self, _state: &Arc<State>) -> anyhow::Result<()> {
        let receiver = self.receiver.lock().unwrap();
        for _event in receiver.iter() {}
        Ok(())
    }
}
