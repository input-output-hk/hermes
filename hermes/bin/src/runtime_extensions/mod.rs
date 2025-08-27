//! Hermes runtime extensions
#![allow(clippy::disallowed_macros)]

use tracing::{span, Level};

mod app_config;
pub(crate) mod bindings;
pub mod hermes;
mod resource_manager;
mod utils;
mod wasi;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    span!(Level::INFO, "Context Span", ctx = %ctx).in_scope(|| {
        hermes::new_context(ctx);
        wasi::new_context(ctx);
    });
}
