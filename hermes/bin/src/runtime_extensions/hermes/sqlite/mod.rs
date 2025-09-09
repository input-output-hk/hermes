//! `SQLite` runtime extension implementation.

mod connection;
mod core;
mod host;
mod state;
mod statement;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}
