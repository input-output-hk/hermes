//! Logging runtime extension implementation.

mod host;
mod log_msg;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}
