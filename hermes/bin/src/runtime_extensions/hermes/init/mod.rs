//! Init runtime extension implementation.

use crate::{event_queue::HermesEventQueueIn, runtime_extensions::state::Stateful};

mod event;

/// State
pub(crate) struct State {}

impl Stateful for State {
    fn new() -> Self {
        State {}
    }
}

impl State {
    /// Emit Init event
    #[allow(clippy::unused_self)]
    pub(crate) fn emit_init_event(
        &self, event_queue_in: &HermesEventQueueIn,
    ) -> anyhow::Result<()> {
        event_queue_in.add(Box::new(event::InitEvent {}))?;
        Ok(())
    }
}
