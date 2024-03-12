//! Init runtime extension implementation.

use crate::{
    app::HermesAppName,
    event as hermes_event,
    event::{HermesEvent, TargetApp, TargetModule},
};

mod event;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}

/// Emit Init event for a provided Hermes app target
pub(crate) fn emit_init_event(target_apps: Vec<HermesAppName>) -> anyhow::Result<()> {
    if !target_apps.is_empty() {
        let init_event = HermesEvent::new(
            event::InitEvent {},
            TargetApp::List(target_apps),
            TargetModule::All,
        );
        hermes_event::queue::send(init_event)?;
    }
    Ok(())
}
