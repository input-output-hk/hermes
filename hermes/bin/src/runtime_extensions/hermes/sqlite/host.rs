//! `SQLite` host implementation for WASM runtime.

use super::{core, state::get_db_state};
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
                let app_state = get_db_state().get_app_state(self.app_name())?;
                let db_id = app_state.create_resource(db_ptr as _);

                Ok(Ok(db_id))
            },
            Err(err) => Ok(Err(err)),
        }
    }
}
