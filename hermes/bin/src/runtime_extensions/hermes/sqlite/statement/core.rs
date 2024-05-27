///! Core functionality implementation for `SQLite` statement object.
use libsqlite3_sys::{
    sqlite3_bind_blob, sqlite3_bind_double, sqlite3_bind_int, sqlite3_bind_int64,
    sqlite3_bind_null, sqlite3_bind_text, sqlite3_column_blob, sqlite3_column_bytes,
    sqlite3_column_double, sqlite3_column_int64, sqlite3_column_text, sqlite3_column_type,
    sqlite3_finalize, sqlite3_step, sqlite3_stmt, SQLITE_BLOB, SQLITE_DONE, SQLITE_ERROR,
    SQLITE_FLOAT, SQLITE_INTEGER, SQLITE_NULL, SQLITE_OK, SQLITE_ROW, SQLITE_TEXT,
    SQLITE_TRANSIENT,
};

use std::os::raw::c_char;

use crate::runtime_extensions::bindings::hermes::sqlite::api::{Errno, Value};

/// Stores application data into parameters of the original SQL.
pub(crate) fn bind(stmt_ptr: *mut sqlite3_stmt, index: i32, value: Value) -> Result<(), Errno> {
    let result = unsafe {
        match value {
            Value::Blob(value) => sqlite3_bind_blob(
                stmt_ptr,
                index,
                value.as_ptr().cast::<std::ffi::c_void>(),
                value.len() as i32,
                SQLITE_TRANSIENT(),
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

                let n_byte = c_value.as_bytes_with_nul().len();

                sqlite3_bind_text(
                    stmt_ptr,
                    index,
                    c_value.as_ptr(),
                    n_byte as i32,
                    SQLITE_TRANSIENT(),
                )
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
pub(crate) fn step(stmt_ptr: *mut sqlite3_stmt) -> Result<(), Errno> {
    let result = unsafe { sqlite3_step(stmt_ptr) };

    if result != SQLITE_DONE && result != SQLITE_ROW {
        Err(result.into())
    } else {
        Ok(())
    }
}

/// Returns information about a single column of the current result row of a query.
pub(crate) fn column(stmt_ptr: *mut sqlite3_stmt, index: i32) -> Result<Value, Errno> {
    let value = unsafe {
        let column_type = sqlite3_column_type(stmt_ptr, index);

        match column_type {
            SQLITE_BLOB => {
                let blob_ptr = sqlite3_column_blob(stmt_ptr, index);
                let blob_len = sqlite3_column_bytes(stmt_ptr, index) as usize;
                let blob_slice = std::slice::from_raw_parts(blob_ptr.cast::<u8>(), blob_len);
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
                let text_slice = std::ffi::CStr::from_ptr(text_ptr.cast::<c_char>());

                let text_string = match text_slice.to_str() {
                    Ok(value) => String::from(value),
                    Err(_) => return Err(SQLITE_ERROR.into()),
                };

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
pub(crate) fn finalize(stmt_ptr: *mut sqlite3_stmt) -> Result<(), Errno> {
    let result = unsafe { sqlite3_finalize(stmt_ptr) };

    if result != SQLITE_OK {
        Err(result.into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::HermesAppName,
        runtime_extensions::hermes::sqlite::{
            connection::core::{self, close, execute, prepare},
            core::open,
        },
    };
    use libsqlite3_sys::*;

    fn init() -> *mut sqlite3 {
        let app_name = HermesAppName(String::from("tmp"));

        open(false, true, app_name).unwrap()
    }

    fn init_value(db_ptr: *mut sqlite3, db_value_type: &str, value: Value) {
        let sql = format!("CREATE TABLE Dummy(Id INTEGER PRIMARY KEY, Value {db_value_type});");
        let sql_cstring = std::ffi::CString::new(sql).unwrap();

        let () = execute(db_ptr, sql_cstring).unwrap();

        let sql = String::from("INSERT INTO Dummy(Value) VALUES(?);");
        let sql_cstring = std::ffi::CString::new(sql).unwrap();

        let stmt_ptr = prepare(db_ptr, sql_cstring).unwrap();

        let () = bind(stmt_ptr, 1, value).unwrap();
        let () = step(stmt_ptr).unwrap();
        let () = finalize(stmt_ptr).unwrap();
    }

    fn get_value(db_ptr: *mut sqlite3) -> Result<Value, Errno> {
        let sql = String::from("SELECT Value FROM Dummy WHERE Id = 1;");
        let sql_cstring = std::ffi::CString::new(sql).unwrap();

        let stmt_ptr = prepare(db_ptr, sql_cstring).unwrap();
        let () = step(stmt_ptr).unwrap();
        let col_result = column(stmt_ptr, 0);
        let () = finalize(stmt_ptr).unwrap();

        col_result
    }

    #[test]
    fn test_value_double() {
        let db_ptr = init();

        let value = Value::Double(3.14159);
        let () = init_value(db_ptr, "REAL", value.clone());
        let value_result = get_value(db_ptr);

        if let (Value::Double(x), Ok(Value::Double(y))) = (value, value_result) {
            assert_eq!(x, y);
        } else {
            panic!();
        }

        close(db_ptr).unwrap();
    }

    #[test]
    fn test_value_bool() {
        let db_ptr = init();

        let value = Value::Int32(1);
        let () = init_value(db_ptr, "BOOLEAN", value.clone());
        let value_result = get_value(db_ptr);

        if let (Value::Int32(x), Ok(Value::Int32(y))) = (value, value_result) {
            assert_eq!(x, y);
        } else {
            panic!();
        }

        close(db_ptr).unwrap();
    }

    #[test]
    fn test_value_int32() {
        let db_ptr = init();

        let value = Value::Int32(i32::MAX);
        let () = init_value(db_ptr, "MEDIUMINT", value.clone());
        let value_result = get_value(db_ptr);

        if let (Value::Int32(x), Ok(Value::Int32(y))) = (value, value_result) {
            assert_eq!(x, y);
        } else {
            panic!();
        }

        close(db_ptr).unwrap();
    }

    #[test]
    fn test_value_int32_nullable() {
        let db_ptr = init();

        let value = Value::Null;
        let () = init_value(db_ptr, "MEDIUMINT", value.clone());
        let value_result = get_value(db_ptr);

        if let (Value::Null, Ok(Value::Null)) = (value, value_result) {
            assert!(true);
        } else {
            panic!();
        }

        close(db_ptr).unwrap();
    }

    #[test]
    fn test_value_int64() {
        let db_ptr = init();

        let value = Value::Int64(i64::MAX);
        let () = init_value(db_ptr, "BIGINT", value.clone());
        let value_result = get_value(db_ptr);

        if let (Value::Int64(x), Ok(Value::Int64(y))) = (value, value_result) {
            assert_eq!(x, y);
        } else {
            panic!();
        }

        close(db_ptr).unwrap();
    }

    #[test]
    fn test_value_text() {
        let db_ptr = init();

        let value = Value::Text(String::from("Hello, World!"));
        let () = init_value(db_ptr, "TEXT", value.clone());
        let value_result = get_value(db_ptr);

        if let (Value::Text(x), Ok(Value::Text(y))) = (value, value_result) {
            assert_eq!(x, y);
        } else {
            panic!();
        }

        close(db_ptr).unwrap();
    }

    #[test]
    fn test_value_blob() {
        let db_ptr = init();

        let value = Value::Blob(vec![1, 2, 3, 4, 5]);
        let () = init_value(db_ptr, "BLOB", value.clone());
        let value_result = get_value(db_ptr);

        if let (Value::Blob(x), Ok(Value::Blob(y))) = (value, value_result) {
            assert_eq!(x, y);
        } else {
            panic!();
        }

        close(db_ptr).unwrap();
    }

    #[test]
    fn test_finalize_simple() {
        let db_ptr = init();

        let sql = String::from("SELECT 1;");
        let sql_cstring = std::ffi::CString::new(sql).unwrap();

        let stmt_ptr = core::prepare(db_ptr, sql_cstring).unwrap();

        let _ = close(db_ptr);
        let result = finalize(stmt_ptr);

        assert!(result.is_ok());
    }
}
