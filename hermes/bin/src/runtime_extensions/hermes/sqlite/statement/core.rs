//! Core functionality implementation for `SQLite` statement object.
use std::os::raw::c_char;

use libsqlite3_sys::{
    sqlite3_bind_blob, sqlite3_bind_double, sqlite3_bind_int, sqlite3_bind_int64,
    sqlite3_bind_null, sqlite3_bind_text, sqlite3_column_blob, sqlite3_column_bytes,
    sqlite3_column_double, sqlite3_column_int64, sqlite3_column_text, sqlite3_column_type,
    sqlite3_finalize, sqlite3_reset, sqlite3_step, sqlite3_stmt, SQLITE_BLOB, SQLITE_DONE,
    SQLITE_FLOAT, SQLITE_INTEGER, SQLITE_NULL, SQLITE_OK, SQLITE_ROW, SQLITE_TEXT,
    SQLITE_TRANSIENT,
};

use crate::runtime_extensions::bindings::hermes::sqlite::api::{Errno, StepResult, Value};

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
pub(crate) fn step(stmt_ptr: *mut sqlite3_stmt) -> Result<StepResult, Errno> {
    let rc = unsafe { sqlite3_step(stmt_ptr) };

    match rc {
        SQLITE_DONE => Ok(StepResult::Done),
        SQLITE_ROW => Ok(StepResult::Row),
        _ => Err(Errno::Sqlite(rc)),
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

/// Reset the prepared statement.
pub(crate) fn reset(stmt_ptr: *mut sqlite3_stmt) -> Result<(), Errno> {
    let rc = unsafe { sqlite3_reset(stmt_ptr) };

    if rc == SQLITE_OK {
        Ok(())
    } else {
        Err(Errno::Sqlite(rc))
    }
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
        let app_name = ApplicationName(TMP_DIR.to_string());
        open(false, true, app_name)
    }

    fn init_value(
        db_ptr: *mut sqlite3,
        db_value_type: &str,
        value: Value,
    ) -> Result<(), Errno> {
        let sql = format!("CREATE TABLE Dummy(Id INTEGER PRIMARY KEY, Value {db_value_type});");
        execute(db_ptr, &sql)?;

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
        match step(stmt_ptr)? {
            StepResult::Row => {
                let val = column(stmt_ptr, 0)?;
                finalize(stmt_ptr)?;
                Ok(val)
            },
            StepResult::Done => {
                finalize(stmt_ptr)?;
                Err(Errno::Sqlite(SQLITE_DONE))
            },
        }
    }

    fn test_single_value(
        db_type: &str,
        value: &Value,
    ) -> Result<(), Errno> {
        let db_ptr = init()?;
        init_value(db_ptr, db_type, value.clone())?;
        let value_result = get_value(db_ptr)?;
        assert_eq!(format!("{value:?}",), format!("{value_result:?}",));
        close(db_ptr)
    }

    #[test]
    fn test_double() -> Result<(), Errno> {
        test_single_value("REAL", &Value::Double(std::f64::consts::PI))
    }
    #[test]
    fn test_bool() -> Result<(), Errno> {
        test_single_value("BOOLEAN", &Value::Int32(1))
    }
    #[test]
    fn test_int32() -> Result<(), Errno> {
        test_single_value("MEDIUMINT", &Value::Int32(i32::MAX))
    }
    #[test]
    fn test_int32_nullable() -> Result<(), Errno> {
        test_single_value("MEDIUMINT", &Value::Null)
    }
    #[test]
    fn test_int64() -> Result<(), Errno> {
        test_single_value("BIGINT", &Value::Int64(i64::MAX))
    }
    #[test]
    fn test_text() -> Result<(), Errno> {
        test_single_value("TEXT", &Value::Text("Hello, World!".to_string()))
    }
    #[test]
    fn test_blob() -> Result<(), Errno> {
        test_single_value("BLOB", &Value::Blob(vec![1, 2, 3, 4, 5]))
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

    #[test]
    fn test_loop_over_rows() -> Result<(), Errno> {
        let db_ptr = init()?;

        let sql = "CREATE TABLE IF NOT EXISTS Dummy(Id INTEGER PRIMARY KEY, Value INTEGER);";
        execute(db_ptr, sql)?;

        // Insert multiple rows
        let sql = "INSERT INTO Dummy(Value) VALUES(?);";
        let stmt_ptr = prepare(db_ptr, sql)?;
        for i in 1..=5 {
            bind(stmt_ptr, 1, Value::Int32(i))?;
            step(stmt_ptr)?;
            reset(stmt_ptr)?;
        }
        finalize(stmt_ptr)?;

        let sql = "SELECT Value FROM Dummy ORDER BY Id ASC;";
        let stmt_ptr = prepare(db_ptr, sql)?;
        let mut collected = Vec::new();

        loop {
            let result = step(stmt_ptr)?;
            match result {
                StepResult::Row => collected.push(column(stmt_ptr, 0)?),
                StepResult::Done => break,
            }
        }
        finalize(stmt_ptr)?;

        for (i, val) in collected.iter().enumerate() {
            match val {
                Value::Int32(v) => assert_eq!(*v, i32::try_from(i).unwrap() + 1),
                _ => panic!("Unexpected value type: {val:?}"),
            }
        }

        close(db_ptr)
    }
}
