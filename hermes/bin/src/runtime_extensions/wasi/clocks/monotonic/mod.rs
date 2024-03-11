//! Monotonic clock runtime extension implementation.

mod host;
mod state;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}
