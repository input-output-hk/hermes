//! Host - WASI IO Implementation

pub(crate) mod error;
pub(crate) mod poll;
pub(crate) mod streams;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    poll::new_context(ctx);
    error::new_context(ctx);
    streams::new_context(ctx);
}
