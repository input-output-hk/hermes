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

        // let inmemory_config = get_app_inmemory_sqlite_db_cfg();
        let persistent_config = get_app_persistent_sqlite_db_cfg();

        let db_path = if memory {
            ":memory:"
        } else {
            "db_name"
        };
        let flags = if readonly {
            SQLITE_OPEN_READONLY
        } else {
            SQLITE_OPEN_CREATE | SQLITE_OPEN_READWRITE
        };

        let result = unsafe {
            sqlite3_open_v2(
                db_path.as_ptr() as *const _,
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
            
            
        } else {
            let db_name = std::ffi::CString::new("main").unwrap();
            let limit_ptr: *const u32 = &persistent_config.max_db_size;

            let rc = unsafe {
                sqlite3_file_control(
                    db_ptr,
                    db_name.as_ptr(),
                    SQLITE_FCNTL_SIZE_LIMIT,
                    limit_ptr as *mut _
                )
            };

            if rc != SQLITE_OK {
                
            }
        }

        Ok(Ok(wasmtime::component::Resource::new_own(db_ptr as u32)))
    }
}
