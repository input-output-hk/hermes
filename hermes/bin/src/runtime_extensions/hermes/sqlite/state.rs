//! Internal state implementation for the `SQLite` module.

use once_cell::sync::Lazy;

use crate::runtime_extensions::{
    bindings::hermes::sqlite::api::{Sqlite, Statement},
    resource_manager::ApplicationResourceManager,
};

/// The object pointer used specifically with C objects like `sqlite3` or `sqlite3_stmt`.
type ObjectPointer = usize;

/// Map of app name to db resource holder
type DbState = ApplicationResourceManager<Sqlite, ObjectPointer>;

/// Map of app name to db statement resource holder
type StatementState = ApplicationResourceManager<Statement, ObjectPointer>;

/// Global state to hold `SQLite` db resources.
static SQLITE_DB_STATE: Lazy<DbState> = Lazy::new(DbState::new);

/// Global state to hold `SQLite` statement resources.
static SQLITE_STATEMENT_STATE: Lazy<StatementState> = Lazy::new(StatementState::new);

/// Get the global state of `SQLite` db resources.
pub(crate) fn get_db_state() -> &'static DbState {
    &SQLITE_DB_STATE
}

/// Get the global state of `SQLite` statement resources.
pub(crate) fn get_statement_state() -> &'static StatementState {
    &SQLITE_STATEMENT_STATE
}
