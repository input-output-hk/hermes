//! Init runtime extension implementation.

use crate::{
    event_queue::{
        event::{HermesEvent, TargetApp, TargetModule},
        HermesEventQueue,
    },
    runtime_extensions::state::Stateful,
};

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
        let init_event = HermesEvent::new(event::InitEvent {}, TargetApp::All, TargetModule::All);
        event_queue.add(init_event)?;
        Ok(())
    }
}
