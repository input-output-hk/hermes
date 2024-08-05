// cspell: words errcode errmsg

//! Core functionality implementation for `SQLite` connection object.

use std::ptr::null_mut;

use libsqlite3_sys::{
    sqlite3, sqlite3_close, sqlite3_errcode, sqlite3_errmsg, sqlite3_exec, sqlite3_prepare_v3,
    sqlite3_stmt, SQLITE_OK,
};
use stringzilla::StringZilla;

use crate::runtime_extensions::bindings::hermes::sqlite::api::{Errno, ErrorInfo};

/// Checks if the provided SQL string contains a `PRAGMA` statement.
/// Generally, `PRAGMA` is intended for internal use only.
pub(crate) fn validate_sql(sql: &str) -> bool {
    sql.to_uppercase().sz_find("PRAGMA ".as_bytes()).is_some()
}

/// Closes a database connection, destructor for `sqlite3`.
pub(crate) fn close(db_ptr: *mut sqlite3) -> Result<(), Errno> {
    let rc = unsafe { sqlite3_close(db_ptr) };

    if rc == SQLITE_OK {
        Ok(())
    } else {
        Err(Errno::Sqlite(rc))
    }
}

/// Retrieves runtime status information about a single database connection.
pub(crate) fn errcode(db_ptr: *mut sqlite3) -> Option<ErrorInfo> {
    let (error_code, error_msg) = unsafe { (sqlite3_errcode(db_ptr), sqlite3_errmsg(db_ptr)) };

    if error_code == SQLITE_OK {
        return None;
    }

    let message = unsafe {
        std::ffi::CStr::from_ptr(error_msg)
            .to_str()
            .map(std::borrow::ToOwned::to_owned)
            .ok()
    };

    message.map(|message| {
        ErrorInfo {
            code: error_code,
            message,
        }
    })
}

/// Compiles SQL text into byte-code that will do the work of querying or updating the
/// database.
pub(crate) fn prepare(db_ptr: *mut sqlite3, sql: &str) -> Result<*mut sqlite3_stmt, Errno> {
    if validate_sql(sql) {
        return Err(Errno::ForbiddenPragmaCommand);
    }

    let sql_cstring = std::ffi::CString::new(sql).map_err(|_| Errno::ConvertingCString)?;

    let mut stmt_ptr: *mut sqlite3_stmt = std::ptr::null_mut();

    let n_byte = sql_cstring.as_bytes_with_nul().len();
    let n_byte = i32::try_from(n_byte).map_err(|_| Errno::ConvertingNumeric)?;

    let rc = unsafe {
        sqlite3_prepare_v3(
            db_ptr,
            sql_cstring.as_ptr(),
            n_byte,
            0,
            &mut stmt_ptr,
            std::ptr::null_mut(),
        )
    };

    if rc == SQLITE_OK {
        Ok(stmt_ptr)
    } else {
        Err(Errno::Sqlite(rc))
    }
}

/// Executes an SQL query directly without preparing it into a statement and returns
/// the result.
pub(crate) fn execute(db_ptr: *mut sqlite3, sql: &str) -> Result<(), Errno> {
    let sql_cstring = std::ffi::CString::new(sql).map_err(|_| Errno::ConvertingCString)?;

    let rc = unsafe { sqlite3_exec(db_ptr, sql_cstring.as_ptr(), None, null_mut(), null_mut()) };

    if rc == SQLITE_OK {
        Ok(())
    } else {
        Err(Errno::Sqlite(rc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::ApplicationName,
        runtime_extensions::hermes::sqlite::{core::open, statement::core::finalize},
    };

    const TMP_DIR: &str = "tmp-dir";

    fn init() -> Result<*mut sqlite3, Errno> {
        let app_name = ApplicationName(String::from(TMP_DIR));

        open(false, true, app_name)
    }

    #[test]
    fn test_validate_pragma() {
        let db_ptr = init().unwrap();

        let sql = "PRAGMA page_size;";
        let stmt_ptr = prepare(db_ptr, sql);

        assert!(matches!(stmt_ptr, Err(Errno::ForbiddenPragmaCommand)));

        let sql = "pragma page_size;";
        let stmt_ptr = prepare(db_ptr, sql);

        assert!(matches!(stmt_ptr, Err(Errno::ForbiddenPragmaCommand)));

        close(db_ptr).unwrap();
    }

    #[test]
    fn test_prepare_simple() {
        let db_ptr = init().unwrap();

        let sql = "SELECT 1;";

        let stmt_ptr = prepare(db_ptr, sql).unwrap();

        finalize(stmt_ptr).unwrap();

        close(db_ptr).unwrap();
    }

    #[test]
    fn test_execute_create_schema_simple() {
        let db_ptr = init()?;

        let create_table_sql = r"
            CREATE TABLE IF NOT EXISTS people (
                id INTEGER PRIMARY KEY,
                name TEXT,
                age INTEGER
            );
        ";

        execute(db_ptr, create_table_sql).unwrap();

        close(db_ptr).unwrap();
    }

    #[test]
    fn test_err_info() {
        let db_ptr = init().unwrap();

        let insert_user_sql = r"
            INSERT INTO user(name, email) VALUES('testing', 'sample');
        ";
        let result = execute(db_ptr, insert_user_sql);

        let err_info = errcode(db_ptr).unwrap();

        close(db_ptr).unwrap();

        assert!(result.is_err());

        assert_eq!(err_info.code, 1);
        assert_eq!(err_info.message, String::from("no such table: user"));
    }

    #[test]
    fn test_execute_create_schema_multiple() {
        let db_ptr = init().unwrap();

        let create_table_sql = r"
            CREATE TABLE user (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL UNIQUE
            );
            CREATE TABLE profile (
                id INTEGER PRIMARY KEY,
                bio TEXT NOT NULL
            );
        ";
        execute(db_ptr, create_table_sql).unwrap();

        let insert_user_sql = r"
            INSERT INTO user(name, email) VALUES('testing', 'sample');
        ";
        execute(db_ptr, insert_user_sql).unwrap();

        let insert_order_sql = r"
            INSERT INTO profile(bio) VALUES('testing');
        ";
        execute(db_ptr, insert_order_sql).unwrap();

        let err_info = errcode(db_ptr);

        assert!(err_info.is_none());

        close(db_ptr).unwrap();
    }

    #[test]
    fn test_close_simple() {
        let db_ptr = init().unwrap();

        close(db_ptr).unwrap();
    }
}
