///! Core functionality implementation for `SQLite` statement object.
use libsqlite3_sys::{SQLITE_BLOB, SQLITE_DONE, SQLITE_ERROR, SQLITE_FLOAT, SQLITE_INTEGER, SQLITE_NULL, SQLITE_OK, SQLITE_TEXT, sqlite3_bind_blob, sqlite3_bind_double, sqlite3_bind_int, sqlite3_bind_int64, sqlite3_bind_null, sqlite3_bind_text, sqlite3_column_bytes, sqlite3_column_double, sqlite3_column_int64, sqlite3_column_text, sqlite3_column_type, sqlite3_column_value, sqlite3_finalize, sqlite3_step, sqlite3_stmt};

use crate::runtime_extensions::bindings::hermes::sqlite::api::{Errno, Value};

/// Stores application data into parameters of the original SQL.
pub(super) fn bind(stmt_ptr: *mut sqlite3_stmt, index: i32, value: Value) -> Result<(), Errno> {
    let result = unsafe {
        match value {
            Value::Blob(value) => sqlite3_bind_blob(
                stmt_ptr,
                index,
                value.as_ptr().cast::<std::ffi::c_void>(),
                value.len() as i32,
                None,
            ),
            Value::Double(value) => sqlite3_bind_double(stmt_ptr, index, value),
            Value::Int32(value) => sqlite3_bind_int(stmt_ptr, index, value),
            Value::Int64(value) => sqlite3_bind_int64(stmt_ptr, index, value),
            Value::Null => sqlite3_bind_null(stmt_ptr, index),
            Value::Text(value) => {
                let c_value = match std::ffi::CString::new(value) {
                    Ok(value) => value,
                    Err(_) => return Err(SQLITE_ERROR.into()),
                };

                sqlite3_bind_text(stmt_ptr, index, c_value.as_ptr(), -1, None)
            },
        }
    };

    if result != SQLITE_OK {
        Err(result.into())
    } else {
        Ok(())
    }
}

/// Advances a statement to the next result row or to completion.
pub(super) fn step(stmt_ptr: *mut sqlite3_stmt) -> Result<(), Errno> {
    let result = unsafe { sqlite3_step(stmt_ptr) };

    if result != SQLITE_DONE {
        Err(result.into())
    } else {
        Ok(())
    }
}

/// Returns information about a single column of the current result row of a query.
pub(super) fn column(stmt_ptr: *mut sqlite3_stmt, index: i32) -> Result<Value, Errno> {
    let value = unsafe {
        let column_type = sqlite3_column_type(stmt_ptr, index);

        match column_type {
            SQLITE_BLOB => {
                let blob_ptr = sqlite3_column_value(stmt_ptr, index);
                let blob_len = sqlite3_column_bytes(stmt_ptr, index);
                let blob_slice =
                    std::slice::from_raw_parts(blob_ptr as *const u8, blob_len as usize);
                Value::Blob(blob_slice.to_vec())
            },
            SQLITE_FLOAT => Value::Double(sqlite3_column_double(stmt_ptr, index)),
            SQLITE_INTEGER => {
                let int_value = sqlite3_column_int64(stmt_ptr, index);
                if int_value >= i64::from(std::i32::MIN) && int_value <= i64::from(std::i32::MAX) {
                    Value::Int32(int_value as i32)
                } else {
                    Value::Int64(int_value)
                }
            },
            SQLITE_NULL => Value::Null,
            SQLITE_TEXT => {
                let text_ptr = sqlite3_column_text(stmt_ptr, index);
                let text_slice = std::ffi::CStr::from_ptr(text_ptr.cast::<i8>());
                let text_string = text_slice.to_string_lossy().into_owned();
                Value::Text(text_string)
            },
            _ => return Err(SQLITE_ERROR.into()),
        }
    };

    Ok(value)
}

/// Destroys a prepared statement object. If the most recent evaluation of the
/// statement encountered no errors or if the statement is never been evaluated,
/// then the function results without errors. If the most recent evaluation of
/// statement failed, then the function results the appropriate error code.
pub(super) fn finalize(stmt_ptr: *mut sqlite3_stmt) -> Result<(), Errno> {
    let result = unsafe { sqlite3_finalize(stmt_ptr) };

    if result != SQLITE_OK {
        Err(result.into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {}
