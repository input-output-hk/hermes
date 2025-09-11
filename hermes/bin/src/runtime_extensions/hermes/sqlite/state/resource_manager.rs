//! Resource manager for SQLite connections and statements.

use std::cell::RefCell;

use crate::{
    app::ApplicationName,
    runtime_extensions::{
        bindings::hermes::sqlite::api::{Sqlite, Statement},
        hermes::sqlite::state::{
            app_not_found_err,
            connection::DbHandle,
            connection_not_found_err,
            manager::{AppSqliteState, SqliteState},
            statement::AppStatement,
            ObjectPointer,
        },
    },
};

thread_local! {
    /// Global state to hold SQLite resources for all applications.
    static SQLITE_STATE: RefCell<SqliteState> = RefCell::new(SqliteState::default());
}

/// Generic function to get application state with a processor closure.
///
/// This is the core function that all other functions use internally.
/// It provides thread-safe access to the `SQLite` state for a specific application.
///
/// # Parameters
///
/// - `application_name`: The name of the application to get state for
/// - `processor`: A closure that processes the optional application state
///
/// # Returns
///
/// The result of the processor closure
fn with_app_state<F, R>(
    application_name: &ApplicationName,
    processor: F,
) -> R
where
    F: FnOnce(Option<&mut AppSqliteState>) -> R,
{
    SQLITE_STATE.with(|state| {
        let mut state = state.borrow_mut();
        processor(state.get_app_state(application_name))
    })
}

/// Generic function to get or create application state with a processor closure.
///
/// This ensures that the application has `SQLite` state available for use.
/// If the application doesn't exist in the state, it will be created with default values.
///
/// # Parameters
///
/// - `application_name`: The name of the application to get or create state for
/// - `processor`: A closure that processes the application state
///
/// # Returns
///
/// The result of the processor closure
fn with_or_create_app_state<F, R>(
    application_name: &ApplicationName,
    processor: F,
) -> R
where
    F: FnOnce(&mut AppSqliteState) -> R,
{
    SQLITE_STATE.with(|state| {
        let mut state = state.borrow_mut();
        processor(state.get_or_create_app_state(application_name))
    })
}

/// Gets the application-specific statement state, creating it if it doesn't exist.
///
/// This function ensures that the application has statement state available for use.
/// If the application doesn't exist in the state, it will be created with default
/// values (empty statement map and address counter starting at 0).
///
/// # Parameters
///
/// - `application_name`: The name of the application to get or create state for
/// - `processor`: A closure that processes the application state
///
/// # Returns
///
/// The result of the processor closure, or an error if processing fails
pub fn get_or_create_statement_app_state<F, R>(
    application_name: &ApplicationName,
    processor: F,
) -> Result<R, wasmtime::Error>
where
    F: FnOnce(&mut AppStatement) -> Result<R, wasmtime::Error>,
{
    with_or_create_app_state(application_name, |app_state| {
        processor(AppSqliteState::statements_mut(app_state))
    })
}

/// High-level convenience functions that provide simple access to resources
/// without requiring complex closure handling.
///
/// Gets a database connection resource for the specified application and database handle.
///
/// This function provides a convenient way to get a connection resource without
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
/// The connection resource, or None if the application or connection is not found
pub fn get_connection_resource(
    application_name: &ApplicationName,
    db_handle: DbHandle,
) -> Option<wasmtime::component::Resource<Sqlite>> {
    with_app_state(application_name, |app_state| {
        match app_state {
            Some(app_state) => app_state.get_connection_resource(db_handle),
            None => None,
        }
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
) -> wasmtime::component::Resource<Sqlite> {
    with_or_create_app_state(application_name, |app_state| {
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
    with_app_state(application_name, |app_state| {
        match app_state {
            Some(app_state) => {
                match app_state.connections.get_connection(db_handle) {
                    Some(connection) => Ok(*connection),
                    None => Err(connection_not_found_err()),
                }
            },
            None => Err(app_not_found_err()),
        }
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
    with_app_state(application_name, |app_state| {
        match app_state {
            Some(app_state) => {
                match app_state.statements.get_object(resource) {
                    Ok(ptr) => Ok(*ptr),
                    Err(e) => Err(e),
                }
            },
            None => {
                Err(wasmtime::Error::msg(
                    "Application not found for statement resource",
                ))
            },
        }
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
    with_app_state(application_name, |app_state| {
        match app_state {
            Some(app_state) => app_state.statements.delete_resource(resource),
            None => {
                Err(wasmtime::Error::msg(
                    "Application not found for statement resource",
                ))
            },
        }
    })
}
