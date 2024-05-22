///! Core functionality implementation for `SQLite` connection object.
use libsqlite3_sys::{
    sqlite3, sqlite3_close_v2, sqlite3_db_status, sqlite3_prepare_v3, sqlite3_step, sqlite3_stmt, sqlite3_finalize,
    SQLITE_DBSTATUS_CACHE_HIT, SQLITE_DBSTATUS_CACHE_MISS, SQLITE_DBSTATUS_CACHE_SPILL,
    SQLITE_DBSTATUS_CACHE_USED, SQLITE_DBSTATUS_CACHE_USED_SHARED, SQLITE_DBSTATUS_CACHE_WRITE,
    SQLITE_DBSTATUS_DEFERRED_FKS, SQLITE_DBSTATUS_LOOKASIDE_HIT,
    SQLITE_DBSTATUS_LOOKASIDE_MISS_FULL, SQLITE_DBSTATUS_LOOKASIDE_MISS_SIZE,
    SQLITE_DBSTATUS_LOOKASIDE_USED, SQLITE_DBSTATUS_SCHEMA_USED, SQLITE_DBSTATUS_STMT_USED,
    SQLITE_DONE, SQLITE_MISUSE, SQLITE_OK,
};
use stringzilla::StringZilla;

use crate::runtime_extensions::bindings::hermes::sqlite::api::{Errno, StatusOptions};

/// Checks if the provided SQL string contains a `PRAGMA` statement.
/// Generally, `PRAGMA` is intended for internal use only.
pub(crate) fn validate_sql(sql: &String) -> bool {
    sql.sz_find("PRAGMA ".as_bytes()).is_some()
}

/// Closes a database connection, destructor for `sqlite3`.
pub(crate) fn close(db_ptr: *mut sqlite3) -> Result<(), Errno> {
    let result = unsafe { sqlite3_close_v2(db_ptr) };

    if result != SQLITE_OK {
        Err(result.into())
    } else {
        Ok(())
    }
}

/// Retrieves runtime status information about a single database connection.
pub(crate) fn status(
    db_ptr: *mut sqlite3, opt: StatusOptions, reset_flag: bool,
) -> Result<(i32, i32), Errno> {
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
        return Err(SQLITE_MISUSE.into());
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
        Err(result.into())
    } else {
        Ok((current_value, highwater_mark))
    }
}

/// Compiles SQL text into byte-code that will do the work of querying or updating the
/// database.
pub(crate) fn prepare(
    db_ptr: *mut sqlite3, sql: std::ffi::CString,
) -> Result<*mut sqlite3_stmt, Errno> {
    let mut stmt_ptr: *mut sqlite3_stmt = std::ptr::null_mut();

    let n_byte = sql.as_bytes_with_nul().len();

    let result = unsafe {
        sqlite3_prepare_v3(
            db_ptr,
            sql.as_ptr(),
            n_byte as i32,
            0,
            &mut stmt_ptr,
            std::ptr::null_mut(),
        )
    };

    if result != SQLITE_OK {
        Err(result.into())
    } else {
        Ok(stmt_ptr)
    }
}

/// Executes an SQL query directly without preparing it into a statement and returns
/// the result.
pub(crate) fn execute(db_ptr: *mut sqlite3, sql: std::ffi::CString) -> Result<(), Errno> {
    let stmt_ptr = prepare(db_ptr, sql)?;

    println!("#######: pass");

    let result = unsafe { sqlite3_step(stmt_ptr) };
    if result != SQLITE_DONE {
        println!("#######: pass {}", result);
        return Err(result.into());
    }

    let result = unsafe { sqlite3_finalize(stmt_ptr) };
    if result != SQLITE_OK {
        return Err(result.into());
    }

    println!("#######: passed");

    Ok(())
}

#[cfg(test)]
mod tests {
    use libsqlite3_sys::*;
    use super::*;
    use crate::{app::HermesAppName, runtime_extensions::hermes::sqlite::core::open};

    fn init() -> *mut sqlite3 {
        let app_name = HermesAppName(String::from("tmp"));

        open(false, true, app_name).unwrap()
    }

    #[test]
    fn test_prepare_simple() {
        let db_ptr = init();

        let sql = String::from("SELECT 1;");
        let sql_cstring = std::ffi::CString::new(sql).unwrap();

        let stmt_ptr = prepare(db_ptr, sql_cstring);

        assert!(stmt_ptr.is_ok())
    }

    #[test]
    fn test_execute_create_schema_simple() {
        let db_ptr = init();

        let create_table_sql = r#"
            CREATE TABLE IF NOT EXISTS people (
                id INTEGER PRIMARY KEY,
                name TEXT,
                age INTEGER
            );
        "#;

        let sql_cstring = std::ffi::CString::new(create_table_sql).unwrap();

        let result = execute(db_ptr, sql_cstring);

        assert!(result.is_ok())
    }

    /* #[test]
    fn test_execute_simple() {
        let db_ptr = init();

        let sql = String::from("SELECT 1;");
        let sql_cstring = std::ffi::CString::new(sql).unwrap();

        let result = execute(db_ptr, sql_cstring);

        println!("##########: {:?}", result);

        assert!(result.is_ok())
    } */

    #[test]
    fn test_close_simple() {
        let db_ptr = init();

        let result = close(db_ptr);

        assert!(result.is_ok())
    }
}
