//! Hermes runtime extensions

pub(crate) mod bindings;
pub mod hermes;
pub(crate) mod wasi;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    hermes::new_context(ctx);
    wasi::new_context(ctx);
}
