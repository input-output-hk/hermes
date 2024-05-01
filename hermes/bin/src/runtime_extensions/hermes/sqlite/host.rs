//! `SQLite` host implementation for WASM runtime.

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
        &mut self, _readonly: bool, _memory: bool,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<Sqlite>, Errno>> {
        todo!()
    }
}
