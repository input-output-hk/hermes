//! Core functionality implementation for `SQLite` statement object.
use std::os::raw::c_char;

use libsqlite3_sys::{
    sqlite3_bind_blob, sqlite3_bind_double, sqlite3_bind_int, sqlite3_bind_int64,
    sqlite3_bind_null, sqlite3_bind_text, sqlite3_column_blob, sqlite3_column_bytes,
    sqlite3_column_double, sqlite3_column_int64, sqlite3_column_text, sqlite3_column_type,
    sqlite3_finalize, sqlite3_step, sqlite3_stmt, SQLITE_BLOB, SQLITE_DONE, SQLITE_FLOAT,
    SQLITE_INTEGER, SQLITE_NULL, SQLITE_OK, SQLITE_ROW, SQLITE_TEXT, SQLITE_TRANSIENT,
};

use crate::runtime_extensions::bindings::hermes::sqlite::api::{Errno, Value};

/// Stores application data into parameters of the original SQL.
pub(crate) fn bind(
    stmt_ptr: *mut sqlite3_stmt,
    index: i32,
    value: Value,
) -> Result<(), Errno> {
    let rc = unsafe {
        match value {
            Value::Blob(value) => {
                let bytes_len = i32::try_from(value.len()).map_err(|_| Errno::ConvertingNumeric)?;

                sqlite3_bind_blob(
                    stmt_ptr,
                    index,
                    value.as_ptr().cast::<std::ffi::c_void>(),
                    bytes_len,
                    SQLITE_TRANSIENT(),
                )
            },
            Value::Double(value) => sqlite3_bind_double(stmt_ptr, index, value),
            Value::Int32(value) => sqlite3_bind_int(stmt_ptr, index, value),
            Value::Int64(value) => sqlite3_bind_int64(stmt_ptr, index, value),
            Value::Null => sqlite3_bind_null(stmt_ptr, index),
            Value::Text(value) => {
                let c_value =
                    std::ffi::CString::new(value).map_err(|_| Errno::ConvertingCString)?;

                let n_byte = c_value.as_bytes_with_nul().len();
                let n_byte = i32::try_from(n_byte).map_err(|_| Errno::ConvertingNumeric)?;

                sqlite3_bind_text(
                    stmt_ptr,
                    index,
                    c_value.as_ptr(),
                    n_byte,
                    SQLITE_TRANSIENT(),
                )
            },
        }
    };

    if rc == SQLITE_OK {
        Ok(())
    } else {
        Err(Errno::Sqlite(rc))
    }
}

/// Advances a statement to the next result row or to completion.
pub(crate) fn step(stmt_ptr: *mut sqlite3_stmt) -> Result<(), Errno> {
    let rc = unsafe { sqlite3_step(stmt_ptr) };

    if rc != SQLITE_DONE && rc != SQLITE_ROW {
        Err(Errno::Sqlite(rc))
    } else {
        Ok(())
    }
}

/// Returns information about a single column of the current result row of a query.
pub(crate) fn column(
    stmt_ptr: *mut sqlite3_stmt,
    index: i32,
) -> Result<Value, Errno> {
    let value = unsafe {
        let column_type = sqlite3_column_type(stmt_ptr, index);

        match column_type {
            SQLITE_BLOB => {
                let blob_ptr = sqlite3_column_blob(stmt_ptr, index);
                let blob_len = sqlite3_column_bytes(stmt_ptr, index);
                let blob_len = usize::try_from(blob_len).map_err(|_| Errno::ConvertingNumeric)?;
                let blob_slice = std::slice::from_raw_parts(blob_ptr.cast::<u8>(), blob_len);
                Value::Blob(blob_slice.to_vec())
            },
            SQLITE_FLOAT => Value::Double(sqlite3_column_double(stmt_ptr, index)),
            SQLITE_INTEGER => {
                let int_value = sqlite3_column_int64(stmt_ptr, index);
                if let Ok(int_value) = i32::try_from(int_value) {
                    Value::Int32(int_value)
                } else {
                    Value::Int64(int_value)
                }
            },
            SQLITE_NULL => Value::Null,
            SQLITE_TEXT => {
                let text_ptr = sqlite3_column_text(stmt_ptr, index);
                let text_slice = std::ffi::CStr::from_ptr(text_ptr.cast::<c_char>());

                let text_string = text_slice
                    .to_str()
                    .map(String::from)
                    .map_err(|_| Errno::ConvertingCString)?;

                Value::Text(text_string)
            },
            _ => return Err(Errno::UnknownColumnType),
        }
    };

    Ok(value)
}

/// Destroys a prepared statement object. If the most recent evaluation of the
/// statement encountered no errors or if the statement is never been evaluated,
/// then the function results without errors. If the most recent evaluation of
/// statement failed, then the function results the appropriate error code.
pub(crate) fn finalize(stmt_ptr: *mut sqlite3_stmt) -> Result<(), Errno> {
    let rc = unsafe { sqlite3_finalize(stmt_ptr) };

    if rc == SQLITE_OK {
        Ok(())
    } else {
        Err(Errno::Sqlite(rc))
    }
}

#[cfg(test)]
mod tests {
    use libsqlite3_sys::*;

    use super::*;
    use crate::{
        app::ApplicationName,
        runtime_extensions::hermes::sqlite::{
            connection::core::{self, close, execute, prepare},
            core::open,
        },
    };

    const TMP_DIR: &str = "tmp-dir";

    fn init() -> Result<*mut sqlite3, Errno> {
        let app_name = ApplicationName(String::from(TMP_DIR));

        open(false, true, app_name)
    }

    fn init_value(
        db_ptr: *mut sqlite3,
        db_value_type: &str,
        value: Value,
    ) -> Result<(), Errno> {
        let sql = format!("CREATE TABLE Dummy(Id INTEGER PRIMARY KEY, Value {db_value_type});");

        execute(db_ptr, sql.as_str())?;

        let sql = "INSERT INTO Dummy(Value) VALUES(?);";

        let stmt_ptr = prepare(db_ptr, sql)?;

        bind(stmt_ptr, 1, value)?;
        step(stmt_ptr)?;
        finalize(stmt_ptr)?;

        Ok(())
    }

    fn get_value(db_ptr: *mut sqlite3) -> Result<Value, Errno> {
        let sql = "SELECT Value FROM Dummy WHERE Id = 1;";

        let stmt_ptr = prepare(db_ptr, sql)?;
        step(stmt_ptr)?;
        let col_result = column(stmt_ptr, 0);
        finalize(stmt_ptr)?;

        col_result
    }

    #[test]
    fn test_value_double() -> Result<(), Errno> {
        let db_ptr = init()?;

        let value = Value::Double(std::f64::consts::PI);
        init_value(db_ptr, "REAL", value.clone())?;
        let value_result = get_value(db_ptr);

        assert!(
            matches!((value, value_result), (Value::Double(x), Ok(Value::Double(y))) if x.eq(&y))
        );

        close(db_ptr)
    }

    #[test]
    fn test_value_bool() -> Result<(), Errno> {
        let db_ptr = init()?;

        let value = Value::Int32(1);
        init_value(db_ptr, "BOOLEAN", value.clone())?;
        let value_result = get_value(db_ptr);

        assert!(matches!((value, value_result), (Value::Int32(x), Ok(Value::Int32(y))) if x == y));

        close(db_ptr)
    }

    #[test]
    fn test_value_int32() -> Result<(), Errno> {
        let db_ptr = init()?;

        let value = Value::Int32(i32::MAX);
        init_value(db_ptr, "MEDIUMINT", value.clone())?;
        let value_result = get_value(db_ptr);

        assert!(matches!((value, value_result), (Value::Int32(x), Ok(Value::Int32(y))) if x == y));

        close(db_ptr)
    }

    #[test]
    fn test_value_int32_nullable() -> Result<(), Errno> {
        let db_ptr = init()?;

        let value = Value::Null;
        init_value(db_ptr, "MEDIUMINT", value.clone())?;
        let value_result = get_value(db_ptr);

        assert!(matches!(
            (value, value_result),
            (Value::Null, Ok(Value::Null))
        ));

        close(db_ptr)
    }

    #[test]
    fn test_value_int64() -> Result<(), Errno> {
        let db_ptr = init()?;

        let value = Value::Int64(i64::MAX);
        init_value(db_ptr, "BIGINT", value.clone())?;
        let value_result = get_value(db_ptr);

        assert!(matches!((value, value_result), (Value::Int64(x), Ok(Value::Int64(y))) if x == y));

        close(db_ptr)
    }

    #[test]
    fn test_value_text() -> Result<(), Errno> {
        let db_ptr = init()?;

        let value = Value::Text(String::from("Hello, World!"));
        init_value(db_ptr, "TEXT", value.clone())?;
        let value_result = get_value(db_ptr);

        assert!(matches!((value, value_result), (Value::Text(x), Ok(Value::Text(y))) if x == y));

        close(db_ptr)
    }

    #[test]
    fn test_value_blob() -> Result<(), Errno> {
        let db_ptr = init()?;

        let value = Value::Blob(vec![1, 2, 3, 4, 5]);
        init_value(db_ptr, "BLOB", value.clone())?;
        let value_result = get_value(db_ptr);

        assert!(matches!((value, value_result), (Value::Blob(x), Ok(Value::Blob(y))) if x == y));

        close(db_ptr)
    }

    #[test]
    fn test_finalize_simple() -> Result<(), Errno> {
        let db_ptr = init()?;

        let sql = "SELECT 1;";

        let stmt_ptr = core::prepare(db_ptr, sql)?;

        let result = finalize(stmt_ptr);

        assert!(result.is_ok());

        close(db_ptr)
    }
}
