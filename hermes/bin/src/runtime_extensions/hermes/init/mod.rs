//! Init runtime extension implementation.

use crate::{
    app::HermesAppName,
    event::{queue::HermesEventQueue, HermesEvent, TargetApp, TargetModule},
};

mod event;

/// Emit Init event for a provided Hermes app target
pub(crate) fn emit_init_event(target_apps: Vec<HermesAppName>) -> anyhow::Result<()> {
    if !target_apps.is_empty() {
        let init_event = HermesEvent::new(
            event::InitEvent {},
            TargetApp::List(target_apps),
            TargetModule::All,
        );
        HermesEventQueue::get_instance()?.add_into_queue(init_event)?;
    }
    Ok(())
}
