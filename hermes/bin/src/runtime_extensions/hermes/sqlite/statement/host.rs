//! `SQLite` statement host implementation for WASM runtime.

use libsqlite3_sys::*;

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::sqlite::api::{Errno, HostStatement, Statement, Value},
};

impl HostStatement for HermesRuntimeContext {
    /// Stores application data into parameters of the original SQL.
    ///
    /// ## Parameters
    ///
    /// - `index`: The index of the SQL parameter to be set.
    /// - `value`: The value to bind to the parameter.
    fn bind(
        &mut self, resource: wasmtime::component::Resource<Statement>, index: u32, value: Value,
    ) -> wasmtime::Result<Result<(), Errno>> {
        let stmt_ptr: *mut sqlite3_stmt = resource.rep() as *mut _;
        let index = index as i32;

        let result = unsafe {
            match value {
                Value::Blob(value) => {
                    sqlite3_bind_blob(
                        stmt_ptr,
                        index,
                        value.as_ptr() as *const std::ffi::c_void,
                        value.len() as i32,
                        None,
                    )
                },
                Value::Double(value) => sqlite3_bind_double(stmt_ptr, index, value),
                Value::Int32(value) => sqlite3_bind_int(stmt_ptr, index, value),
                Value::Int64(value) => sqlite3_bind_int64(stmt_ptr, index, value),
                Value::Null => sqlite3_bind_null(stmt_ptr, index),
                Value::Text(value) => {
                    let c_value = std::ffi::CString::new(value)
                        .map_err(|_| wasmtime::Error::msg("Failed to convert string to CString"))?;

                    sqlite3_bind_text(stmt_ptr, index, c_value.as_ptr(), -1, None)
                },
            }
        };

        if result != SQLITE_OK {
            Ok(Err(result.into()))
        } else {
            Ok(Ok(()))
        }
    }

    /// Advances a statement to the next result row or to completion.
    ///
    /// After a prepared statement has been prepared, this function must be called one or
    /// more times to evaluate the statement.
    fn step(
        &mut self, resource: wasmtime::component::Resource<Statement>,
    ) -> wasmtime::Result<Result<(), Errno>> {
        let stmt_ptr: *mut sqlite3_stmt = resource.rep() as *mut _;

        let result = unsafe { sqlite3_step(stmt_ptr) };

        if result != SQLITE_OK {
            Ok(Err(result.into()))
        } else {
            Ok(Ok(()))
        }
    }

    /// Returns information about a single column of the current result row of a query.
    ///
    /// If the SQL statement does not currently point to a valid row, or if the column
    /// index is out of range, the result is undefined.
    ///
    /// ## Parameters
    ///
    /// - `index`: The index of the column for which information should be returned. The
    ///   leftmost column of the result set has the index 0.
    ///
    /// ## Returns
    ///
    /// The value of a result column in a specific data format.
    fn column(
        &mut self, _resource: wasmtime::component::Resource<Statement>, _index: u32,
    ) -> wasmtime::Result<Result<Value, Errno>> {
        todo!()
    }

    /// Destroys a prepared statement object. If the most recent evaluation of the
    /// statement encountered no errors or if the statement is never been evaluated,
    /// then the function results without errors. If the most recent evaluation of
    /// statement failed, then the function results the appropriate error code.
    ///
    /// The application must finalize every prepared statement in order to avoid resource
    /// leaks. It is a grievous error for the application to try to use a prepared
    /// statement after it has been finalized. Any use of a prepared statement after
    /// it has been finalized can result in undefined and undesirable behavior such as
    /// segfaults and heap corruption.
    fn finalize(
        &mut self, resource: wasmtime::component::Resource<Statement>,
    ) -> wasmtime::Result<Result<(), Errno>> {
        let stmt_ptr: *mut sqlite3_stmt = resource.rep() as *mut _;

        let result = unsafe { sqlite3_finalize(stmt_ptr) };

        if result != SQLITE_OK {
            Ok(Err(result.into()))
        } else {
            Ok(Ok(()))
        }
    }

    fn drop(&mut self, _rep: wasmtime::component::Resource<Statement>) -> wasmtime::Result<()> {
        todo!()
    }
}
