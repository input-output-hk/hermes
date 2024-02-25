//! Init runtime extension implementation.

use crate::{event_queue::HermesEventQueue, runtime_extensions::state::Stateful};

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
    pub(crate) fn emit_init_event(&self, event_queue: &HermesEventQueue) -> anyhow::Result<()> {
        event_queue.add(Box::new(event::InitEvent {}))?;
        Ok(())
    }
}
