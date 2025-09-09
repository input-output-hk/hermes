//! Internal state implementation for the `SQLite` module.

/// Connection state management for `SQLite` database resources
pub(crate) mod connection;

/// Statement state management for `SQLite` prepared statement resources  
pub(crate) mod statement;

/// Unified resource manager for SQLite connections and statements
pub(crate) mod manager;

/// Resource manager providing unified access to SQLite state
pub(crate) mod resource_manager;

/// The object pointer used specifically with C objects like `sqlite3` or `sqlite3_stmt`.
pub(super) type ObjectPointer = usize;

/// Creates a standardized error for when an application is not found in the state.
///
/// This function returns a consistent error message that indicates the application
/// needs to be initialized before accessing connection resources.
///
/// # Returns
///
/// A `wasmtime::Error` with a descriptive message about the missing application
pub(crate) fn app_not_found_err() -> wasmtime::Error {
    wasmtime::Error::msg(
        "Application not found for connection resource, need to add application first by calling `create_connection_resource`",
    )
}
