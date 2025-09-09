//! `SQLite` host implementation for WASM runtime.

use super::core;
use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::hermes::sqlite::api::{Errno, Host, Sqlite},
        hermes::sqlite::state::{
            connection::DbHandle, get_db_app_state_with, get_or_create_db_app_state_with,
        },
    },
};

impl Host for HermesRuntimeContext {
    /// Opens a connection to a new or existing `SQLite` database.
    ///
    /// ## Parameters
    ///
    /// - `readonly`: If set to true, the database is opened in read-only mode. An error
    ///   is returned if the database doesn't already exist.
    /// - `memory`: If set to true, the database will be opened as an in-memory database.
    ///
    /// ## Returns
    ///
    /// If the database is opened (and/or created) successfully, then the `sqlite3` object
    /// is returned. Otherwise an error code is returned.
    fn open(
        &mut self,
        readonly: bool,
        memory: bool,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<Sqlite>, Errno>> {
        let db_handle = DbHandle::from_readonly_and_memory(readonly, memory);
        if let Some(resource) = get_db_app_state_with(self.app_name(), |app_state| {
            app_state.and_then(|app_state| app_state.get_connection_resource(db_handle))
        }) {
            return Ok(Ok(resource));
        }

        match core::open(readonly, memory, self.app_name().clone()) {
            Ok(db_ptr) => {
                let db_id = get_or_create_db_app_state_with(self.app_name(), |app_state| {
                    app_state.create_connection_resource(db_handle, db_ptr as _)
                });

                Ok(Ok(db_id))
            },
            Err(err) => Ok(Err(err)),
        }
    }
}
