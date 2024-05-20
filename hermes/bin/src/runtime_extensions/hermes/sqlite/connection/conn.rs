//! Core functionality implementation fo SQLite connection object.

use libsqlite3_sys::*;
use stringzilla::StringZilla;

use crate::runtime_extensions::bindings::hermes::sqlite::api::{Errno, StatusOptions};

/// Checks if the provided SQL string contains a `PRAGMA` statement.
/// Generally, `PRAGMA` is intended for internal use only.
pub(super) fn validate_sql(sql: &String) -> bool {
  sql.sz_find("PRAGMA ".as_bytes()).is_some()
}

/// Closes a database connection, destructor for `sqlite3`.
pub(super) fn close(db_ptr: *mut sqlite3) -> Result<(), Errno> {
    let result = unsafe { sqlite3_close_v2(db_ptr) };

    if result != SQLITE_OK {
        Err(result.into())
    } else {
        Ok(())
    }
}

/// Retrieves runtime status information about a single database connection.
pub(super) fn status(
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
pub(super) fn prepare(
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
pub(super) fn execute(db_ptr: *mut sqlite3, sql: std::ffi::CString) -> Result<(), Errno> {
    let stmt_ptr = prepare(db_ptr, sql)?;

    let result = unsafe { sqlite3_step(stmt_ptr) };
    if result != SQLITE_DONE {
        return Err(result.into());
    }

    Ok(())
}
