//! Hermes runtime extensions

#![allow(clippy::todo)]

use tracing::{span, Level};

pub(crate) mod app_config;
pub(crate) mod bindings;
pub mod hermes;
pub(crate) mod wasi;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    span!(Level::INFO, "Context Span", ctx = ?ctx).in_scope(|| {
        hermes::new_context(ctx);
        wasi::new_context(ctx);
    });
}
