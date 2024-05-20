//! `SQLite` host implementation for WASM runtime.

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

        let db_ptr = match core::open(readonly, memory, app_name) {
            Ok(db_ptr) => db_ptr,
            Err(err) => {
                return match err {
                    core::OpenError::InvalidInMemoryConfig => Err(wasmtime::Error::msg(
                        "In-memory config is not set for a in-memory option",
                    )),
                    core::OpenError::InvalidPersistentConfig => Err(wasmtime::Error::msg(
                        "Persistent config is not set for a non-memory option",
                    )),
                    core::OpenError::MissingDatabaseNameForPersistentConfig => Err(
                        wasmtime::Error::msg("Database name is not set for a database file config"),
                    ),
                    core::OpenError::FailedOpeningDatabase => Err(wasmtime::Error::msg(
                        "Error opening a connection to the database",
                    )),
                    core::OpenError::FailedSettingDatabaseSize => {
                        Err(wasmtime::Error::msg("Error setting database size"))
                    },
                    core::OpenError::SQLiteError(errno) => Ok(Err(errno)),
                }
            },
        };

        Ok(Ok(wasmtime::component::Resource::new_own(db_ptr as u32)))
    }
}
