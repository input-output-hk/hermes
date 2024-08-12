//! IO Streams runtime extension implementation.

mod host;
mod state;

pub(crate) use state::get_intput_streams_state;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}
