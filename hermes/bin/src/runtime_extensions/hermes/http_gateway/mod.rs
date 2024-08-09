//! HTTP Gateway

use gateway_task::spawn;

mod event;
mod gateway_task;
/// Gateway routing logic
mod routing;

///  State.
static STATE: once_cell::sync::Lazy<()> = once_cell::sync::Lazy::new(|| {
    spawn();
});

/// New context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {
    // Init state event
    let _ = STATE;
}
