//! `SQLite` host implementation for WASM runtime.

use libsqlite3_sys::*;

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{app_config::*, bindings::hermes::sqlite::api::{Errno, Host, Sqlite}},
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
        let mut db_ptr: *mut sqlite3 = std::ptr::null_mut();

        let (db_path, config) = if memory {
            let inmemory_config = match get_app_inmemory_sqlite_db_cfg() {
                Some(config) => config,
                None => return Err(wasmtime::Error::msg(
                    "In-memory config is not set for a in-memory option",
                ))
            };

            (":memory:".into(), inmemory_config)
        } else {
            let persistent_config = match get_app_persistent_sqlite_db_cfg() {
                Some(config) => config,
                None => return Err(wasmtime::Error::msg(
                    "Persistent config is not set for a non-memory option",
                ))
            };

            let db_name = match &persistent_config.db_file {
                Some(db_name) => db_name.clone(),
                None => return Err(wasmtime::Error::msg(
                    "Database name is not set for a database file config",
                ))
            };

            (db_name, persistent_config)
        };
        let flags = if readonly {
            SQLITE_OPEN_READONLY
        } else {
            SQLITE_OPEN_CREATE | SQLITE_OPEN_READWRITE
        };

        let result = unsafe {
            sqlite3_open_v2(
                db_path.as_str().as_ptr() as *const _,
                &mut db_ptr,
                flags,
                std::ptr::null(),
            )
        };


        if result != SQLITE_OK {
            return Ok(Err(result.into()));
        } else if db_ptr.is_null() {
            return Err(wasmtime::Error::msg(
                "Error opening a connection to the database",
            ));
        }

        // config database size limitation
        if memory {
            let size_limit = config.max_db_size as i64;

            let rc = unsafe {
                sqlite3_file_control(
                    db_ptr,
                    "main\0".as_ptr() as *const i8,
                    SQLITE_FCNTL_SIZE_LIMIT,
                    size_limit as *mut std::ffi::c_void
                )
            };
            
            if rc != SQLITE_OK {
                return Err(wasmtime::Error::msg(
                    "Error setting database size",
                ));
            }
        } else {
            // FIXME: convert bytes to page
            let pragma_stmt = format!("PRAGMA max_page_count = {}", config.max_db_size);
            let c_pragma_stmt = std::ffi::CString::new(pragma_stmt).map_err(|_| wasmtime::Error::msg("Failed to convert string to CString"))?;

            // TODO: handle size
            let rc = unsafe {
                sqlite3_exec(
                    db_ptr,
                    c_pragma_stmt.as_ptr(),
                    None,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };

            if rc != SQLITE_OK {
                return Err(wasmtime::Error::msg(
                    "Error setting database size",
                ));
            }
        }

        Ok(Ok(wasmtime::component::Resource::new_own(db_ptr as u32)))
    }
}
