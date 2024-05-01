//! `SQLite` runtime extension implementation.

mod host;
mod sqlite;
mod statement;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    sqlite::new_context(ctx);
    statement::new_context(ctx);
}
