//! Hermes runtime extensions implementations - WASI standard extensions

pub(crate) mod cli;
pub(crate) mod clocks;
pub(crate) mod context;
pub(crate) mod descriptors;
pub(crate) mod filesystem;
pub(crate) mod http;
pub(crate) mod io;
pub(crate) mod random;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    cli::new_context(ctx);
    clocks::new_context(ctx);
    filesystem::new_context(ctx);
    http::new_context(ctx);
    io::new_context(ctx);
    random::new_context(ctx);
}
