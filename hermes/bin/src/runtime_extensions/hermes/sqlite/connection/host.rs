//! `SQLite` connection object host implementation for WASM runtime.

use libsqlite3_sys::*;
use stringzilla::StringZilla;

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::sqlite::api::{
        Errno, HostSqlite, Sqlite, Statement, StatusOptions,
    },
};

impl HostSqlite for HermesRuntimeContext {
    /// Closes a database connection, destructor for `sqlite3`.
    ///
    /// Ideally, applications should finalize all prepared statements associated with the
    /// `sqlite3` object prior to attempting to close the object. If the database
    /// connection is associated with unfinalized prepared statements,
    /// then the function will leave the database connection open and return the `busy`
    /// error code.
    ///
    /// If an `sqlite3` object is destroyed while a transaction is open, the transaction
    /// is automatically rolled back.
    fn close(
        &mut self, resource: wasmtime::component::Resource<Sqlite>,
    ) -> wasmtime::Result<Result<(), Errno>> {
        let db_ptr: *mut sqlite3 = resource.rep() as *mut _;

        let result = unsafe { sqlite3_close_v2(db_ptr) };

        if result != SQLITE_OK {
            Ok(Err(result.into()))
        } else {
            Ok(Ok(()))
        }
    }

    /// Retrieves runtime status information about a single database connection.
    ///
    /// ## Parameters
    ///
    /// - `opt`: An integer constant, taken from the set of `status-options`, that
    ///   determines the parameter to interrogate.
    /// - `reset-flag`: If is true, then the highest instantaneous value is reset back
    ///   down to the current value.
    ///
    /// ## Returns
    ///
    /// A tuple of the current value of the requested parameter, and the highest
    /// instantaneous value on success, and an error code on failure.
    fn status(
        &mut self, resource: wasmtime::component::Resource<Sqlite>, opt: StatusOptions,
        reset_flag: bool,
    ) -> wasmtime::Result<Result<(i32, i32), Errno>> {
        let db_ptr: *mut sqlite3 = resource.rep() as *mut _;

        let status_code = if opt.contains(StatusOptions::LOOKASIDE_USED) {
            SQLITE_DBSTATUS_LOOKASIDE_USED
        } else if opt.contains(StatusOptions::CACHE_USED) {
            SQLITE_DBSTATUS_CACHE_USED
        } else if opt.contains(StatusOptions::SCHEMA_USED) {
            SQLITE_DBSTATUS_SCHEMA_USED
        } else if opt.contains(StatusOptions::STMT_USED) {
            SQLITE_DBSTATUS_STMT_USED
        } else if opt.contains(StatusOptions::LOOKASIDE_HIT) {
            SQLITE_DBSTATUS_LOOKASIDE_HIT
        } else if opt.contains(StatusOptions::LOOKASIDE_MISS_FULL) {
            SQLITE_DBSTATUS_LOOKASIDE_MISS_FULL
        } else if opt.contains(StatusOptions::LOOKASIDE_MISS_SIZE) {
            SQLITE_DBSTATUS_LOOKASIDE_MISS_SIZE
        } else if opt.contains(StatusOptions::CACHE_HIT) {
            SQLITE_DBSTATUS_CACHE_HIT
        } else if opt.contains(StatusOptions::CACHE_MISS) {
            SQLITE_DBSTATUS_CACHE_MISS
        } else if opt.contains(StatusOptions::CACHE_WRITE) {
            SQLITE_DBSTATUS_CACHE_WRITE
        } else if opt.contains(StatusOptions::DEFERRED_FKS) {
            SQLITE_DBSTATUS_DEFERRED_FKS
        } else if opt.contains(StatusOptions::CACHE_USED_SHARED) {
            SQLITE_DBSTATUS_CACHE_USED_SHARED
        } else if opt.contains(StatusOptions::CACHE_SPILL) {
            SQLITE_DBSTATUS_CACHE_SPILL
        } else {
            return Err(wasmtime::Error::msg("Invalid option"))
        };
    
        let mut current_value = 0;
        let mut highwater_mark = 0;
    
        let result = unsafe {
            sqlite3_db_status(
                db_ptr,
                status_code,
                &mut current_value,
                &mut highwater_mark,
                reset_flag.into(),
            )
        };
    
        if result != SQLITE_OK {
            Ok(Err(result.into()))
        } else {
            Ok(Ok((current_value, highwater_mark)))
        }
    }

    /// Compiles SQL text into byte-code that will do the work of querying or updating the
    /// database.
    ///
    /// ## Parameters
    ///
    /// - `db`: Database handle.
    /// - `sql`: SQL statement, UTF-8 encoded.
    ///
    /// ## Returns
    ///
    /// A compiled prepared statement that can be executed using `sqlite3_step()`.
    /// If there is an error or the input text contains no SQL (if the input is an empty
    /// string or a comment) then an error code is returned.
    fn prepare(
        &mut self, resource: wasmtime::component::Resource<Sqlite>, sql: String,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<Statement>, Errno>> {
        if sql.sz_find("PRAGMA ".as_bytes()).is_some() {
            return Err(wasmtime::Error::msg("PRAGMA statement is not allowed"))
        }

        let db_ptr: *mut sqlite3 = resource.rep() as *mut _;
        let mut stmt_ptr: *mut sqlite3_stmt = std::ptr::null_mut();

        let sql_cstring = std::ffi::CString::new(sql)
            .map_err(|_| wasmtime::Error::msg("Failed to convert SQL string to CString"))?;
        let n_byte = sql_cstring.as_bytes_with_nul().len();

        let result = unsafe {
            sqlite3_prepare_v3(
                db_ptr,
                sql_cstring.as_ptr(),
                n_byte as i32,
                0,
                &mut stmt_ptr,
                std::ptr::null_mut(),
            )
        };

        if result != SQLITE_OK {
            Ok(Err(result.into()))
        } else if stmt_ptr.is_null() {
            Err(wasmtime::Error::msg("Error preparing a database statement"))
        } else {
            Ok(Ok(wasmtime::component::Resource::new_own(stmt_ptr as u32)))
        }
    }

    /// Executes an SQL query directly without preparing it into a statement and returns
    /// the result.
    ///
    /// ## Parameters
    ///
    /// - `sql`: SQL statement, UTF-8 encoded.
    fn execute(
        &mut self, resource: wasmtime::component::Resource<Sqlite>, sql: String,
    ) -> wasmtime::Result<Result<(), Errno>> {
        if sql.sz_find("PRAGMA ".as_bytes()).is_some() {
            return Err(wasmtime::Error::msg("PRAGMA statement is not allowed"))
        }
        
        // prepare stage
        let db_ptr: *mut sqlite3 = resource.rep() as *mut _;
        let mut stmt_ptr: *mut sqlite3_stmt = std::ptr::null_mut();

        let sql_cstring = std::ffi::CString::new(sql)
            .map_err(|_| wasmtime::Error::msg("Failed to convert SQL string to CString"))?;
        let n_byte = sql_cstring.as_bytes_with_nul().len();

        let result = unsafe {
            sqlite3_prepare_v3(
                db_ptr,
                sql_cstring.as_ptr(),
                n_byte as i32,
                0,
                &mut stmt_ptr,
                std::ptr::null_mut(),
            )
        };

        if result != SQLITE_OK {
            return Ok(Err(result.into()))
        }

        // step stage
        let result = unsafe { sqlite3_step(stmt_ptr) };

        if result != SQLITE_DONE {
            return Ok(Err(result.into()))
        }
        
        Ok(Ok(()))
    }

    fn drop(&mut self, _rep: wasmtime::component::Resource<Sqlite>) -> wasmtime::Result<()> {
        todo!()
    }
}
