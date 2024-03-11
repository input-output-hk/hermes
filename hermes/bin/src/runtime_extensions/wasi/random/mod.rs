//! Host - WASI - Random implementations

pub(crate) mod insecure;
pub(crate) mod insecure_seed;
pub(crate) mod secure;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    insecure::new_context(ctx);
    insecure_seed::new_context(ctx);
    secure::new_context(ctx);
}
