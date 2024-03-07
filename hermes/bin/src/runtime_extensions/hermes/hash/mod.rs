//! Hash runtime extension implementation.

mod blake2b;
mod host;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}

// `State` is obsolete, needs to be removed.
// If needed, it can be replaced with `new_context`
