//! Internal state implementation for the `SQLite` module.

/// Connection state management for `SQLite` database resources
pub(crate) mod connection;

/// Statement state management for `SQLite` prepared statement resources  
pub(crate) mod statement;

use crate::{
    app::ApplicationName,
    runtime_extensions::hermes::sqlite::state::{
        connection::{AppConnections, DbState},
        statement::{AppStatement, StatementState},
    },
};
use std::{cell::RefCell, collections::HashMap};

/// The object pointer used specifically with C objects like `sqlite3` or `sqlite3_stmt`.
pub(super) type ObjectPointer = usize;

thread_local! {
    /// Global state to hold `SQLite` db resources.
    static SQLITE_DB_STATE: RefCell<DbState<'static>> = RefCell::new(DbState {
        apps: HashMap::new(),
    });

    /// Global state to hold `SQLite` statement resources.
    static SQLITE_STATEMENT_STATE: RefCell<StatementState> = RefCell::new(StatementState::new());
}

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
pub(super) fn get_db_app_state_with<F, R>(
    application_name: &ApplicationName,
    processor: F,
) -> R
where
    F: FnOnce(Option<&mut AppConnections<'_>>) -> R,
{
    SQLITE_DB_STATE.with_borrow_mut(|db_state| processor(db_state.apps.get_mut(application_name)))
}

/// Gets the application-specific database connection state for processing, creating it if it doesn't exist.
///
/// This function provides thread-safe access to the global database state,
/// allowing callers to process the connection state for a specific application.
/// If the application doesn't exist, it will be created with default values.
///
/// # Parameters
///
/// - `application_name`: The name of the application to get or create state for
/// - `processor`: A closure that processes the application state
///
/// # Returns
///
/// The result of the processor closure
pub(super) fn get_or_create_db_app_state_with<F, R>(
    application_name: &ApplicationName,
    processor: F,
) -> R
where
    F: FnOnce(&mut AppConnections<'_>) -> R,
{
    SQLITE_DB_STATE.with_borrow_mut(|db_state| {
        let app_state = db_state.apps.entry(application_name.clone()).or_default();
        processor(app_state)
    })
}

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

/// Manager for `SQLite` statement state operations.
///
/// This struct provides thread-safe access to application-specific statement state.
/// It manages the lifecycle of prepared statement resources and ensures proper
/// isolation between different applications running in the WASM runtime.
pub(crate) struct StatementStateManager;

impl StatementStateManager {
    /// Gets the application-specific statement state for processing, if it exists.
    ///
    /// This method provides thread-safe access to the statement state for a specific
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
    pub(crate) fn get_app_state<F, R>(
        application_name: &ApplicationName,
        processor: F,
    ) -> Result<R, wasmtime::Error>
    where
        F: FnOnce(Option<&mut AppStatement>) -> Result<R, wasmtime::Error>,
    {
        SQLITE_STATEMENT_STATE
            .with_borrow_mut(|statement_state| processor(statement_state.get_mut(application_name)))
    }

    /// Gets the application-specific statement state, creating it if it doesn't exist.
    ///
    /// This method ensures that the application has statement state available for use.
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
    pub(crate) fn get_or_create_app_state<F, R>(
        application_name: &ApplicationName,
        processor: F,
    ) -> Result<R, wasmtime::Error>
    where
        F: FnOnce(&mut AppStatement) -> Result<R, wasmtime::Error>,
    {
        SQLITE_STATEMENT_STATE.with_borrow_mut(|statement_state| {
            let app_state = statement_state
                .entry(application_name.clone())
                .or_insert_with(|| AppStatement {
                    statements: HashMap::new(),
                    available_address: 0,
                });
            processor(app_state)
        })
    }
}
