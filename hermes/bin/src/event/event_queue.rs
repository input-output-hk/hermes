//! Hermes event queue implementation.

use std::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};

use super::HermesEvent;

#[derive(thiserror::Error, Debug, Clone)]
#[error("Failed to add event into the event queue. Event queue is closed.")]
pub(crate) struct QueueClosed;

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
    pub(crate) fn add(&self, event: HermesEvent) -> anyhow::Result<()> {
        self.sender.send(event).map_err(|_| QueueClosed)?;
        Ok(())
    }
}

impl Iterator for &HermesEventQueue {
    type Item = HermesEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.receiver.lock().unwrap().try_recv().ok()
    }
}
