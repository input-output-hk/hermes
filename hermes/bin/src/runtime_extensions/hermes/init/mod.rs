//! Init runtime extension implementation.

use crate::{event_queue::HermesEventQueueIn, runtime_extensions::state::Stateful};

mod event;

/// State
pub(crate) struct State {
    /// Hermes Event Queue
    event_queue_in: HermesEventQueueIn,
}

impl Stateful for State {
    fn new(event_queue_in: &HermesEventQueueIn) -> Self {
        State {
            event_queue_in: event_queue_in.clone(),
        }
    }
}

impl State {
    /// Init event
    pub(crate) fn emit_init_event(&self) -> anyhow::Result<()> {
        self.event_queue_in.add(Box::new(event::InitEvent {}))?;
        Ok(())
    }
}
