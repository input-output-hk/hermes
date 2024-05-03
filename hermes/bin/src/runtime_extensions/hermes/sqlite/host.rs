//! `SQLite` host implementation for WASM runtime.

use libsqlite3_sys::*;

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
        let mut db: *mut sqlite3 = std::ptr::null_mut();
        let db_path = if memory {
            ":memory:"
        } else {
            "your_database_path.db"
        };
        let flags = if readonly {
            SQLITE_OPEN_READONLY
        } else {
            SQLITE_OPEN_CREATE | SQLITE_OPEN_READWRITE
        };

        let result = unsafe {
            sqlite3_open_v2(
                db_path.as_ptr() as *const _,
                &mut db,
                flags,
                std::ptr::null(),
            )
        };

        if result != SQLITE_OK {
            return Ok(Err(result.into()));
        } else if db.is_null() {
            return Err(wasmtime::Error::msg(
                "Error opening a connection to the database",
            ));
        } else {
            return Ok(Ok(wasmtime::component::Resource::new_own(db as u32)));
        }
    }
}
