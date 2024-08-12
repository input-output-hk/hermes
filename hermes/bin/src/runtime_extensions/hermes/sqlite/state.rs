//! Internal state implementation for the `SQLite` module.

use once_cell::sync::Lazy;

use crate::runtime_extensions::{
    bindings::hermes::sqlite::api::{Sqlite, Statement},
    resource_manager::ApplicationResourceManager,
};

/// The object pointer used specifically with C objects like `sqlite3` or `sqlite3_stmt`.
pub(super) type ObjectPointer = usize;

/// Map of app name to db resource holder
pub(super) type DbState = ApplicationResourceManager<Sqlite, ObjectPointer>;

/// Map of app name to db statement resource holder
pub(super) type StatementState = ApplicationResourceManager<Statement, ObjectPointer>;

/// Global state to hold `SQLite` db resources.
static SQLITE_DB_STATE: Lazy<DbState> = Lazy::new(DbState::new);

/// Global state to hold `SQLite` statement resources.
static SQLITE_STATEMENT_STATE: Lazy<StatementState> = Lazy::new(StatementState::new);

/// Get the global state of `SQLite` db resources.
pub(super) fn get_db_state() -> &'static DbState {
    &SQLITE_DB_STATE
}

/// Get the global state of `SQLite` statement resources.
pub(super) fn get_statement_state() -> &'static StatementState {
    &SQLITE_STATEMENT_STATE
}
