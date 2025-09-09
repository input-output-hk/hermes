//! Unified resource manager for SQLite connections and statements.

use std::collections::HashMap;

use crate::{
    app::ApplicationName,
    runtime_extensions::hermes::sqlite::state::{
        connection::AppConnections, statement::AppStatement,
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
    /// Gets a mutable reference to the connection state.
    pub(crate) fn connections_mut(&mut self) -> &mut AppConnections {
        &mut self.connections
    }

    /// Gets a mutable reference to the statement state.
    pub(crate) fn statements_mut(&mut self) -> &mut AppStatement {
        &mut self.statements
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
    /// Gets or creates the `SQLite` state for the specified application.
    ///
    /// # Parameters
    ///
    /// - `application_name`: The name of the application to get or create state for
    ///
    /// # Returns
    ///
    /// A mutable reference to the application's `SQLite` state
    pub(crate) fn get_or_create_app_state(
        &mut self,
        application_name: &ApplicationName,
    ) -> &mut AppSqliteState {
        self.apps.entry(application_name.clone()).or_default()
    }

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
