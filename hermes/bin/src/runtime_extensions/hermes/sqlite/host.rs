//! `SQLite` host implementation for WASM runtime.

use anyhow::Ok;

use super::core;
use crate::{
    app::HermesAppName,
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
        // TODO: use actual app name for this
        let app_name = HermesAppName(String::from("tmp"));

        Ok(core::open(readonly, memory, app_name)
            .map(|db_ptr| wasmtime::component::Resource::new_own(db_ptr as u32)))
    }
}
