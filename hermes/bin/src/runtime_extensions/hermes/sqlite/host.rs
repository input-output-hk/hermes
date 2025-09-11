//! `SQLite` host implementation for WASM runtime.

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::hermes::sqlite::api::{Errno, Host, Sqlite},
        hermes::sqlite::{
            core,
            state::{connection::DbHandle, resource_manager},
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

        // Check if connection already exists
        if let Some(resource) =
            resource_manager::get_connection_resource(self.app_name(), db_handle)
        {
            return Ok(Ok(resource));
        }

        // Create new connection
        match core::open(readonly, memory, self.app_name().clone()) {
            Ok(db_ptr) => {
                let resource = resource_manager::create_connection_resource(
                    self.app_name(),
                    db_handle,
                    db_ptr as _,
                );
                Ok(Ok(resource))
            },
            Err(err) => Ok(Err(err)),
        }
    }
}
