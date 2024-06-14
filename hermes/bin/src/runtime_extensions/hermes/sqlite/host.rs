//! `SQLite` host implementation for WASM runtime.

use super::{core, state};
use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::sqlite::api::{Errno, Host, Sqlite},
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
        &mut self, readonly: bool, memory: bool,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<Sqlite>, Errno>> {
        match core::open(readonly, memory, self.app_name().clone()) {
            Ok(db_ptr) => {
                let db_id = state::InternalState::get_or_create_resource(self.app_name().clone())
                    .get_db_state()
                    .add_object(db_ptr as _)
                    .ok_or_else(|| {
                        wasmtime::Error::msg("Internal state error while calling `open`")
                    })?;

                Ok(Ok(wasmtime::component::Resource::new_own(db_id)))
            },
            Err(err) => Ok(Err(err)),
        }
    }
}
