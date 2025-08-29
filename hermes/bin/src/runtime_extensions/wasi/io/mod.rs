//! Host - WASI IO Implementation

pub(crate) mod error;
pub(crate) mod poll;
pub(crate) mod streams;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    streams::new_context(ctx);
}
