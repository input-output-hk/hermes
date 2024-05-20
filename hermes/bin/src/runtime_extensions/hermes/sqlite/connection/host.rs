//! `SQLite` connection object host implementation for WASM runtime.

use libsqlite3_sys::*;

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::sqlite::api::{
        Errno, HostSqlite, Sqlite, Statement, StatusOptions,
    },
};

use super::core;

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

        Ok(core::close(db_ptr))
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

        Ok(core::status(db_ptr, opt, reset_flag))
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
        if core::validate_sql(&sql) {
            return Err(wasmtime::Error::msg("PRAGMA statement is not allowed"));
        }

        let db_ptr: *mut sqlite3 = resource.rep() as *mut _;

        let sql_cstring = std::ffi::CString::new(sql)
            .map_err(|_| wasmtime::Error::msg("Failed to convert SQL string to CString"))?;

        let result = core::prepare(db_ptr, sql_cstring);

        match result {
            Ok(stmt_ptr) => {
                if stmt_ptr.is_null() {
                    Err(wasmtime::Error::msg("Error preparing a database statement"))
                } else {
                    Ok(Ok(wasmtime::component::Resource::new_own(stmt_ptr as u32)))
                }
            },
            Err(errno) => Ok(Err(errno)),
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
        if core::validate_sql(&sql) {
            return Err(wasmtime::Error::msg("PRAGMA statement is not allowed"));
        }

        let db_ptr: *mut sqlite3 = resource.rep() as *mut _;

        let sql_cstring = std::ffi::CString::new(sql)
            .map_err(|_| wasmtime::Error::msg("Failed to convert SQL string to CString"))?;

        Ok(core::execute(db_ptr, sql_cstring))
    }

    fn drop(&mut self, _rep: wasmtime::component::Resource<Sqlite>) -> wasmtime::Result<()> {
        todo!()
    }
}
