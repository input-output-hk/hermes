//! Resource manager for SQLite connections and statements.

use std::cell::RefCell;

use crate::{
    app::ApplicationName,
    runtime_extensions::{
        bindings::hermes::sqlite::api::{Sqlite, Statement},
        hermes::sqlite::state::{
            ObjectPointer, app_not_found_err,
            connection::DbHandle,
            connection_not_found_err,
            manager::{AppSqliteState, SqliteState},
        },
    },
};

thread_local! {
    /// Thread-local state to hold SQLite resources for all applications.
    static SQLITE_STATE: RefCell<SqliteState> = RefCell::new(SqliteState::default());
}

/// Initializes application state for `SQLite` resources.
///
/// This function creates a new application state entry if it doesn't exist.
/// It should be called during the init event to ensure the application state
/// is properly initialized before any `SQLite` operations are performed.
///
/// # Parameters
///
/// - `application_name`: The name of the application to initialize state for
pub(crate) fn init_app_state(application_name: &ApplicationName) {
    SQLITE_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.apps.entry(application_name.clone()).or_default();
    });
}

/// Tries to get application state if it exists.
///
/// This function provides thread-safe access to the `SQLite` state for a specific
/// application. Returns `None` if the application state doesn't exist.
///
/// # Parameters
///
/// - `application_name`: The name of the application to get state for
/// - `processor`: A closure that processes the optional application state
///
/// # Returns
///
/// The result of the processor closure
fn try_get_app_state_with<F, R>(
    application_name: &ApplicationName,
    processor: F,
) -> R
where
    F: FnOnce(Option<&mut AppSqliteState>) -> R,
{
    SQLITE_STATE.with(|state| {
        // Safe to borrow_mut because all `SQLite` state access goes through
        // `try_get_app_state_with` and `get_app_state_with` only,
        // preventing multiple borrows from occurring simultaneously.
        let mut state = state.borrow_mut();
        processor(state.get_app_state(application_name))
    })
}

/// Gets application state (expects it to exist from init).
///
/// This function provides thread-safe access to the `SQLite` state for a specific
/// application. The application state should have been initialized during the init event.
///
/// # Parameters
///
/// - `application_name`: The name of the application to get state for
/// - `processor`: A closure that processes the application state
///
/// # Returns
///
/// The result of the processor closure, or an error if the application state doesn't
/// exist
fn get_app_state_with<F, R>(
    application_name: &ApplicationName,
    processor: F,
) -> Result<R, wasmtime::Error>
where
    F: FnOnce(&mut AppSqliteState) -> R,
{
    SQLITE_STATE.with(|state| {
        // Safe to borrow_mut because all SQLite state access goes through
        // `try_get_app_state_with` and `get_app_state_with` only,
        // preventing multiple borrows from occurring simultaneously.
        let mut state = state.borrow_mut();
        match state.get_app_state(application_name) {
            Some(app_state) => Ok(processor(app_state)),
            None => Err(wasmtime::Error::msg(format!(
                "Application '{application_name}' state not found - must be initialized in init event"
            ))),
        }
    })
}

/// High-level convenience functions that provide simple access to resources
/// without requiring complex closure handling.
///
/// Gets a database connection resource for the specified application and database handle.
///
/// # Parameters
///
/// - `application_name`: The name of the application
/// - `db_handle`: The database handle to get the connection for
///
/// # Returns
///
/// The connection resource, or None if the application or connection is not found
pub fn try_get_connection_resource(
    application_name: &ApplicationName,
    db_handle: DbHandle,
) -> Option<wasmtime::component::Resource<Sqlite>> {
    try_get_app_state_with(application_name, |app_state| match app_state {
        Some(app_state) => app_state.get_connection_resource(db_handle),
        None => None,
    })
}

/// Creates a new database connection resource for the specified application and database
/// handle.
///
/// This function provides a convenient way to create a connection resource without
/// duplicating error handling logic.
///
/// # Parameters
///
/// - `application_name`: The name of the application
/// - `db_handle`: The database handle for the connection
/// - `db_ptr`: The pointer to the database connection object
///
/// # Returns
///
/// A new connection resource
pub fn create_connection_resource(
    application_name: &ApplicationName,
    db_handle: DbHandle,
    db_ptr: ObjectPointer,
) -> Result<wasmtime::component::Resource<Sqlite>, wasmtime::Error> {
    get_app_state_with(application_name, |app_state| {
        app_state.create_connection_resource(db_handle, db_ptr)
    })
}

/// Gets a database connection pointer for the specified application and database handle.
///
/// This function provides a convenient way to get a connection pointer without
/// duplicating error handling logic. It properly distinguishes between
/// "application not found" and "connection not found" errors.
///
/// # Parameters
///
/// - `application_name`: The name of the application
/// - `db_handle`: The database handle to get the connection for
///
/// # Returns
///
/// The connection pointer, or an error if the application or connection is not found
pub fn get_connection_pointer(
    application_name: &ApplicationName,
    db_handle: DbHandle,
) -> Result<ObjectPointer, wasmtime::Error> {
    try_get_app_state_with(application_name, |app_state| match app_state {
        Some(app_state) => match app_state.connections.get_connection(db_handle) {
            Some(connection) => Ok(*connection),
            None => Err(connection_not_found_err()),
        },
        None => Err(app_not_found_err()),
    })
}

/// Gets a statement pointer for the specified application and statement resource.
///
/// This function provides a convenient way to get a statement pointer without
/// duplicating error handling logic. It properly distinguishes between
/// "application not found" and "statement not found" errors.
///
/// # Parameters
///
/// - `application_name`: The name of the application
/// - `resource`: The statement resource to get the pointer for
///
/// # Returns
///
/// The statement pointer, or an error if the application or statement is not found
pub fn get_statement_pointer(
    application_name: &ApplicationName,
    resource: &wasmtime::component::Resource<Statement>,
) -> Result<ObjectPointer, wasmtime::Error> {
    try_get_app_state_with(application_name, |app_state| match app_state {
        Some(app_state) => match app_state.statements.get_object(resource) {
            Ok(ptr) => Ok(*ptr),
            Err(e) => Err(e),
        },
        None => Err(wasmtime::Error::msg(
            "Application not found for statement resource",
        )),
    })
}

/// Creates a new statement resource for the specified application and statement pointer.
///
/// This function provides a convenient way to create a statement resource without
/// duplicating error handling logic.
///
/// # Parameters
///
/// - `application_name`: The name of the application
/// - `stmt_ptr`: The pointer to the statement object
///
/// # Returns
///
/// A new statement resource
pub fn create_statement_resource(
    application_name: &ApplicationName,
    stmt_ptr: ObjectPointer,
) -> Result<wasmtime::component::Resource<Statement>, wasmtime::Error> {
    get_app_state_with(application_name, |app_state| {
        app_state
            .statements_mut()
            .create_statement_resource(stmt_ptr)
    })
}

/// Deletes a statement resource and returns its pointer.
///
/// This function provides a convenient way to delete a statement resource without
/// duplicating error handling logic.
///
/// # Parameters
///
/// - `application_name`: The name of the application
/// - `resource`: The statement resource to delete
///
/// # Returns
///
/// The statement pointer that was deleted, or an error if the application or statement is
/// not found
pub fn delete_statement_resource(
    application_name: &ApplicationName,
    resource: &wasmtime::component::Resource<Statement>,
) -> Result<ObjectPointer, wasmtime::Error> {
    try_get_app_state_with(application_name, |app_state| match app_state {
        Some(app_state) => app_state.statements.delete_resource(resource),
        None => Err(wasmtime::Error::msg(
            "Application not found for statement resource",
        )),
    })
}
