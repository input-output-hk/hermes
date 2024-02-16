//! Hermes event queue implementation.

use std::sync::mpsc::{channel, Receiver, Sender};

pub(crate) mod event;

/// Initialize Hermes event queue
pub(crate) fn new() -> (HermesEventQueueIn, HermesEventQueueOut) {
    let (sender, receiver) = channel();
    (HermesEventQueueIn(sender), HermesEventQueueOut(receiver))
}

///
pub(crate) struct HermesEventQueueIn(Sender<Box<dyn event::HermesEventPayload>>);

///
pub(crate) struct HermesEventQueueOut(Receiver<Box<dyn event::HermesEventPayload>>);

impl Iterator for HermesEventQueueOut {
    type Item = Box<dyn event::HermesEventPayload>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.recv().ok()
    }
}
