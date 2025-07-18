//! Init runtime extension implementation.

use crate::{
    app::ApplicationName,
    event as hermes_event,
    event::{HermesEvent, TargetApp, TargetModule},
};

mod event;
mod host;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}

/// Emit Init event for a provided Hermes app target
pub(crate) fn emit_init_event(target_app: ApplicationName) -> anyhow::Result<()> {
    let init_event = HermesEvent::new(
        event::InitEvent {},
        TargetApp::List(vec![target_app]),
        TargetModule::All,
    );
    hermes_event::queue::send(init_event)?;
    Ok(())
}
