//! `SQLite` runtime extension implementation.

mod connection;
mod core;
mod host;
mod state;
mod statement;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    connection::new_context(ctx);
    statement::new_context(ctx);
}
