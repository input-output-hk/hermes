// cspell: words errcode errmsg

//! Core functionality implementation for `SQLite` connection object.

use std::ptr::null_mut;

use libsqlite3_sys::{
    SQLITE_OK, sqlite3, sqlite3_close, sqlite3_errcode, sqlite3_errmsg, sqlite3_exec,
    sqlite3_prepare_v3, sqlite3_stmt,
};
use stringzilla::stringzilla::StringZillableBinary;

use crate::runtime_extensions::{
    bindings::hermes::sqlite::api::{Errno, ErrorInfo},
    hermes::sqlite::kernel,
};

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

/// Same as [`close`] but additionally removes all sqlite files with
/// [`kernel::DbPaths::remove_all`].
pub(crate) fn close_and_remove_all(db_ptr: *mut sqlite3) -> anyhow::Result<()> {
    let paths = kernel::DbPaths::main(db_ptr)?;
    close(db_ptr)?;
    paths.remove_all()?;
    Ok(())
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
pub(crate) fn prepare(
    db_ptr: *mut sqlite3,
    sql: &str,
) -> Result<*mut sqlite3_stmt, Errno> {
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
            &raw mut stmt_ptr,
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
pub(crate) fn execute(
    db_ptr: *mut sqlite3,
    sql: &str,
) -> Result<(), Errno> {
    if validate_sql(sql) {
        return Err(Errno::ForbiddenPragmaCommand);
    }

    let sql_cstring = std::ffi::CString::new(sql).map_err(|_| Errno::ConvertingCString)?;

    let rc = unsafe { sqlite3_exec(db_ptr, sql_cstring.as_ptr(), None, null_mut(), null_mut()) };

    if rc == SQLITE_OK {
        Ok(())
    } else {
        Err(Errno::Sqlite(rc))
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use serial_test::file_serial;

    use super::*;
    use crate::{
        app::ApplicationName,
        runtime_extensions::{
            bindings::hermes::sqlite::api::Value,
            hermes::sqlite::{
                kernel::{self, open},
                statement::core::{column, finalize, step},
            },
        },
    };

    const TMP_DIR: &str = "tmp-dir";

    fn init() -> Result<*mut sqlite3, Errno> {
        let app_name = ApplicationName(String::from(TMP_DIR));

        open(false, true, app_name)
    }

    fn init_fs(app_name: String) -> Result<*mut sqlite3, Errno> {
        let app_name = ApplicationName(app_name);

        open(false, false, app_name)
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

        let sql = "pragma page_size;";
        let result = execute(db_ptr, sql);

        assert!(matches!(result, Err(Errno::ForbiddenPragmaCommand)));

        let sql = "PRAGMA page_size;";
        let result = execute(db_ptr, sql);

        assert!(matches!(result, Err(Errno::ForbiddenPragmaCommand)));

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
        let db_ptr = init().unwrap();

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

    #[test]
    #[file_serial]
    fn test_close_and_remove_all_simple() {
        let app_name = TMP_DIR.to_owned();
        let db_ptr = init_fs(app_name).unwrap();
        let paths = kernel::DbPaths::main(db_ptr).unwrap();

        let database_created = std::fs::exists(&paths.database).unwrap();
        let _journal_created = std::fs::exists(&paths.journal).unwrap();
        let _wal_created = std::fs::exists(&paths.wal).unwrap();

        close_and_remove_all(db_ptr).unwrap();

        let database_remained = std::fs::exists(&paths.database).unwrap();
        let journal_remained = std::fs::exists(&paths.database).unwrap();
        let wal_remained = std::fs::exists(&paths.database).unwrap();

        assert!(database_created);
        assert!(!database_remained);
        assert!(!journal_remained);
        assert!(!wal_remained);
    }

    #[test]
    #[file_serial]
    fn test_multiple_threads_does_not_conflict() {
        fn task(app_name: String) {
            let db_ptr = init_fs(app_name).unwrap();
            let statement = prepare(
                db_ptr,
                r"
                    UPDATE counter
                    SET value = value + 1
                    RETURNING value;
                ",
            )
            .expect("failed to prepare statement");
            step(statement).expect("failed to make a step");
            let Value::Int32(_counter) = column(statement, 0).expect("failed to get value") else {
                panic!("invalid type");
            };
            finalize(statement).expect("failed to finalize statement");
            close(db_ptr).expect("failed to close connection");
        }

        // Running db in memory and in file mode at the same time
        // causes issues during test run
        const APP_NAME: &str = "counter-app";

        let db_ptr = init_fs(APP_NAME.to_string()).unwrap();
        execute(
            db_ptr,
            r"
                CREATE TABLE IF NOT EXISTS counter (
                    value INTEGER
                );
                ",
        )
        .expect("failed to create  table");
        execute(db_ptr, "INSERT INTO counter(value) VALUES(0);").expect("failed to insert value");
        close(db_ptr).expect("failed to close connection");

        let mut handlers = vec![];
        for _ in 0..100 {
            let app_name = APP_NAME.to_string();
            handlers.push(std::thread::spawn(move || task(app_name)));
        }

        for handler in handlers {
            handler.join().expect("failed to join handler");
        }
    }
}
