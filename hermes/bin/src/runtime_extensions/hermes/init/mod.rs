//! Init runtime extension implementation.

use crate::event::{queue::HermesEventQueue, HermesEvent, TargetApp, TargetModule};

mod event;

/// Emit Init event for a provided Hermes app target
pub(crate) fn emit_init_event(event_queue: &HermesEventQueue) -> anyhow::Result<()> {
    let init_event = HermesEvent::new(event::InitEvent {}, TargetApp::All, TargetModule::All);
    event_queue.add_into_queue(init_event)?;
    Ok(())
}
