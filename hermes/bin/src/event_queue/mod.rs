//! Hermes event queue implementation.

use std::sync::mpsc::{channel, Receiver, Sender};

pub(crate) mod event;

/// Initialize Hermes event queue
pub(crate) fn new() -> (HermesEventQueueIn, HermesEventQueueOut) {
    let (sender, receiver) = channel();
    (HermesEventQueueIn(sender), HermesEventQueueOut(receiver))
}

///
#[derive(Clone)]
pub(crate) struct HermesEventQueueIn(Sender<Box<dyn event::HermesEventPayload>>);

impl HermesEventQueueIn {
    ///
    pub(crate) fn add(&self, event: Box<dyn event::HermesEventPayload>) -> anyhow::Result<()> {
        self.0.send(event).map_err(|_| {
            anyhow::anyhow!("Failed to add event into the event queue. Event queue is closed")
        })?;
        Ok(())
    }
}

///
pub(crate) struct HermesEventQueueOut(Receiver<Box<dyn event::HermesEventPayload>>);

impl Iterator for HermesEventQueueOut {
    type Item = Box<dyn event::HermesEventPayload>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.recv().ok()
    }
}
