//! Resource manager for SQLite connections and statements.

use std::cell::RefCell;

use crate::{
    app::ApplicationName,
    runtime_extensions::hermes::sqlite::state::{
        connection::AppConnections,
        manager::{AppSqliteState, SqliteState},
        statement::AppStatement,
    },
};

thread_local! {
    /// Global state to hold SQLite resources for all applications.
    static SQLITE_STATE: RefCell<SqliteState> = RefCell::new(SqliteState::default());
}

/// Gets the application-specific `SQLite` state for processing, if it exists.
///
/// This function provides thread-safe access to the `SQLite` state for a specific
/// application. The processor closure receives `None` if the application doesn't
/// have any `SQLite` state initialized.
///
/// # Parameters
///
/// - `application_name`: The name of the application to get state for
/// - `processor`: A closure that processes the optional application state
///
/// # Returns
///
/// The result of the processor closure
pub fn get_app_state<F, R>(
    application_name: &ApplicationName,
    processor: F,
) -> R
where
    F: FnOnce(Option<&mut AppSqliteState>) -> R,
{
    SQLITE_STATE.with_borrow_mut(|state| processor(state.get_app_state(application_name)))
}

/// Gets the application-specific `SQLite` state, creating it if it doesn't exist.
///
/// This function ensures that the application has `SQLite` state available for use.
/// If the application doesn't exist in the state, it will be created with default
/// values.
///
/// # Parameters
///
/// - `application_name`: The name of the application to get or create state for
/// - `processor`: A closure that processes the application state
///
/// # Returns
///
/// The result of the processor closure
pub fn get_or_create_app_state<F, R>(
    application_name: &ApplicationName,
    processor: F,
) -> R
where
    F: FnOnce(&mut AppSqliteState) -> R,
{
    SQLITE_STATE.with_borrow_mut(|state| processor(state.get_or_create_app_state(application_name)))
}

/// Legacy compatibility functions for existing code.
/// Gets the application-specific database connection state for processing.
///
/// This function provides thread-safe access to the global database state,
/// allowing callers to process the connection state for a specific application.
/// The processor closure receives `None` if the application doesn't exist.
///
/// # Parameters
///
/// - `application_name`: The name of the application to get state for
/// - `processor`: A closure that processes the optional application state
///
/// # Returns
///
/// The result of the processor closure
pub fn get_db_app_state_with<F, R>(
    application_name: &ApplicationName,
    processor: F,
) -> R
where
    F: FnOnce(Option<&mut AppConnections>) -> R,
{
    get_app_state(application_name, |app_state| {
        processor(app_state.map(AppSqliteState::connections_mut))
    })
}

/// Gets the application-specific database connection state for processing, creating it if
/// it doesn't exist.
///
/// This function ensures that the application has database connection state available for
/// use. If the application doesn't exist in the state, it will be created with default
/// values.
///
/// # Parameters
///
/// - `application_name`: The name of the application to get or create state for
/// - `processor`: A closure that processes the application state
///
/// # Returns
///
/// The result of the processor closure
pub fn get_or_create_db_app_state_with<F, R>(
    application_name: &ApplicationName,
    processor: F,
) -> R
where
    F: FnOnce(&mut AppConnections) -> R,
{
    get_or_create_app_state(application_name, |app_state| {
        processor(AppSqliteState::connections_mut(app_state))
    })
}

/// Gets the application-specific statement state for processing, if it exists.
///
/// This function provides thread-safe access to the statement state for a specific
/// application. The processor closure receives `None` if the application doesn't
/// have any statement state initialized.
///
/// # Parameters
///
/// - `application_name`: The name of the application to get state for
/// - `processor`: A closure that processes the optional application state
///
/// # Returns
///
/// The result of the processor closure, or an error if processing fails
pub fn get_statement_app_state<F, R>(
    application_name: &ApplicationName,
    processor: F,
) -> Result<R, wasmtime::Error>
where
    F: FnOnce(Option<&mut AppStatement>) -> Result<R, wasmtime::Error>,
{
    get_app_state(application_name, |app_state| {
        processor(app_state.map(AppSqliteState::statements_mut))
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
    get_or_create_app_state(application_name, |app_state| {
        processor(AppSqliteState::statements_mut(app_state))
    })
}
