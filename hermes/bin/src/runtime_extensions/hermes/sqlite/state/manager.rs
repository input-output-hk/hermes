//! Unified resource manager for SQLite connections and statements.

use std::collections::HashMap;

use crate::{
    app::ApplicationName,
    runtime_extensions::{
        bindings::hermes::sqlite::api::Sqlite,
        hermes::sqlite::state::{
            ObjectPointer,
            connection::{AppConnections, DbHandle},
            statement::AppStatement,
        },
    },
};

/// Unified application state for `SQLite` resources.
///
/// This struct manages both database connections and prepared statements
/// for a single application, providing a centralized interface for resource
/// management.
#[derive(Default)]
pub(crate) struct AppSqliteState {
    /// Database connection state
    pub(crate) connections: AppConnections,
    /// Prepared statement state
    pub(crate) statements: AppStatement,
}

impl AppSqliteState {
    /// Gets a mutable reference to the statement state.
    pub(crate) fn statements_mut(&mut self) -> &mut AppStatement {
        &mut self.statements
    }

    /// Gets a connection resource for the specified database handle.
    pub(crate) fn get_connection_resource(
        &self,
        db_handle: DbHandle,
    ) -> Option<wasmtime::component::Resource<Sqlite>> {
        self.connections.get_connection_resource(db_handle)
    }

    /// Creates a new connection resource and stores the connection pointer.
    pub(crate) fn create_connection_resource(
        &mut self,
        db_handle: DbHandle,
        db_ptr: ObjectPointer,
    ) -> wasmtime::component::Resource<Sqlite> {
        self.connections
            .create_connection_resource(db_handle, db_ptr)
    }
}

/// Global `SQLite` state container for all applications.
///
/// This struct holds the unified `SQLite` state for all applications running
/// in the WASM runtime, providing isolation between different applications.
#[derive(Default)]
pub(crate) struct SqliteState {
    /// Map of application names to their `SQLite` state
    pub(crate) apps: HashMap<ApplicationName, AppSqliteState>,
}

impl SqliteState {
    /// Gets the `SQLite` state for the specified application, if it exists.
    ///
    /// # Parameters
    ///
    /// - `application_name`: The name of the application to get state for
    ///
    /// # Returns
    ///
    /// An optional mutable reference to the application's `SQLite` state
    pub(crate) fn get_app_state(
        &mut self,
        application_name: &ApplicationName,
    ) -> Option<&mut AppSqliteState> {
        self.apps.get_mut(application_name)
    }
}
