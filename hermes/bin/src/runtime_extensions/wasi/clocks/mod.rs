//! Host - WASI - Clock implementations

mod monotonic;
mod state;
mod wall;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    monotonic::new_context(ctx);
    wall::new_context(ctx);
}
