/// ! Core functionality implementation for `SQLite` connection object.
use libsqlite3_sys::{
    sqlite3, sqlite3_close_v2, sqlite3_errcode, sqlite3_errmsg, sqlite3_finalize, sqlite3_prepare_v3,
    sqlite3_step, sqlite3_stmt, SQLITE_DONE, SQLITE_OK,
};
use stringzilla::StringZilla;

use crate::runtime_extensions::bindings::hermes::sqlite::api::{Errno, Error};

/// Checks if the provided SQL string contains a `PRAGMA` statement.
/// Generally, `PRAGMA` is intended for internal use only.
pub(crate) fn validate_sql(sql: &str) -> bool {
    sql.sz_find("PRAGMA ".as_bytes()).is_some()
}

/// Closes a database connection, destructor for `sqlite3`.
pub(crate) fn close(db_ptr: *mut sqlite3) -> Result<(), Errno> {
    let rc = unsafe { sqlite3_close_v2(db_ptr) };

    if rc != SQLITE_OK {
        Err(Errno::Sqlite(rc))
    } else {
        Ok(())
    }
}

/// Retrieves runtime status information about a single database connection.
pub(crate) fn errcode(
    db_ptr: *mut sqlite3
) -> Error {
    let (error_code, error_msg) = unsafe {
        (
            sqlite3_errcode(db_ptr),
            sqlite3_errmsg(db_ptr)
        )
    };

    let message = unsafe {
        std::ffi::CString::from_raw(error_msg as *mut i8).into_string()
            .map_err(|_| wasmtime::Error::msg("Failed to convert SQL string to CString"))?
    };

    Error {
        code: error_code,
        message
    }
}

/// Compiles SQL text into byte-code that will do the work of querying or updating the
/// database.
pub(crate) fn prepare(
    db_ptr: *mut sqlite3, sql: std::ffi::CString,
) -> Result<*mut sqlite3_stmt, Errno> {
    let mut stmt_ptr: *mut sqlite3_stmt = std::ptr::null_mut();

    let n_byte = sql.as_bytes_with_nul().len();

    let rc = unsafe {
        sqlite3_prepare_v3(
            db_ptr,
            sql.as_ptr(),
            n_byte as i32,
            0,
            &mut stmt_ptr,
            std::ptr::null_mut(),
        )
    };

    if rc != SQLITE_OK {
        Err(Errno::Sqlite(rc))
    } else {
        Ok(stmt_ptr)
    }
}

/// Executes an SQL query directly without preparing it into a statement and returns
/// the result.
pub(crate) fn execute(db_ptr: *mut sqlite3, sql: std::ffi::CString) -> Result<(), Errno> {
    let stmt_ptr = prepare(db_ptr, sql)?;

    let rc = unsafe { sqlite3_step(stmt_ptr) };
    if rc != SQLITE_DONE {
        return Err(Errno::Sqlite(rc));
    }

    let rc = unsafe { sqlite3_finalize(stmt_ptr) };
    if rc != SQLITE_OK {
        return Err(Errno::Sqlite(rc));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::HermesAppName,
        runtime_extensions::hermes::sqlite::{core::open, statement::core::finalize},
    };

    const TMP_DIR: &str = "tmp-dir";

    fn init() -> *mut sqlite3 {
        let app_name = HermesAppName(String::from(TMP_DIR));

        open(false, true, app_name).unwrap()
    }

    #[test]
    fn test_prepare_simple() {
        let db_ptr = init();

        let sql = String::from("SELECT 1;");
        let sql_cstring = std::ffi::CString::new(sql).unwrap();

        let stmt_ptr = prepare(db_ptr, sql_cstring);

        if let Ok(stmt_ptr) = stmt_ptr {
            let _ = close(db_ptr);
            let _ = finalize(stmt_ptr);
        }

        assert!(stmt_ptr.is_ok());
    }

    #[test]
    fn test_config_simple() {
        let db_ptr = init();

        let before_schema_status_result = status(db_ptr, StatusOptions::SCHEMA_USED, false);

        let create_table_sql = r"
            CREATE TABLE IF NOT EXISTS people (
                id INTEGER PRIMARY KEY,
                name TEXT
            );
        ";

        let sql_cstring = std::ffi::CString::new(create_table_sql).unwrap();

        let () = execute(db_ptr, sql_cstring).unwrap();

        let after_schema_status_result = status(db_ptr, StatusOptions::SCHEMA_USED, false);

        let _ = close(db_ptr);

        if let (Ok((after_value, _)), Ok((before_value, _))) =
            (after_schema_status_result, before_schema_status_result)
        {
            assert!(before_value == 0 && after_value > 0);
        } else {
            panic!()
        }
    }

    #[test]
    fn test_execute_create_schema_simple() {
        let db_ptr = init();

        let create_table_sql = r"
            CREATE TABLE IF NOT EXISTS people (
                id INTEGER PRIMARY KEY,
                name TEXT,
                age INTEGER
            );
        ";

        let sql_cstring = std::ffi::CString::new(create_table_sql).unwrap();

        let result = execute(db_ptr, sql_cstring);

        let _ = close(db_ptr);

        assert!(result.is_ok());
    }

    #[test]
    fn test_close_simple() {
        let db_ptr = init();

        let result = close(db_ptr);

        let _ = close(db_ptr);

        assert!(result.is_ok());
    }
}
