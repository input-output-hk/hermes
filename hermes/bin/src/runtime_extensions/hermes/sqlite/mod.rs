//! `SQLite` runtime extension implementation.

mod connection;
mod core;
mod host;
mod state;
mod statement;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    state::get_db_state().add_app(ctx.app_name().clone());
    state::get_statement_state().add_app(ctx.app_name().clone());
}
