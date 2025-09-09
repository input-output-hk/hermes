//! Core functionality implementation for the `SQLite` open function.

use std::{
    ffi::{c_int, c_void, CString},
    time::Duration,
};

use libsqlite3_sys::{
    sqlite3, sqlite3_busy_handler, sqlite3_exec, sqlite3_open_v2, sqlite3_soft_heap_limit64,
    sqlite3_wal_autocheckpoint, SQLITE_OK, SQLITE_OPEN_CREATE, SQLITE_OPEN_NOMUTEX,
    SQLITE_OPEN_READONLY, SQLITE_OPEN_READWRITE,
};
use rand::random;

use crate::{
    app::ApplicationName,
    runtime_extensions::{
        app_config::{get_app_in_memory_sqlite_db_cfg, get_app_persistent_sqlite_db_cfg},
        bindings::hermes::sqlite::api::Errno,
    },
};

/// The default page size of `SQLite`.
const PAGE_SIZE: u32 = 4_096;
/// Max delay for sql query to retry.
const MAX_DELAY_MS: u64 = 30000;
/// Jitter to avoid all clients retrying in lock-step.
const JITTER_MS: u64 = MAX_DELAY_MS * 2 / 10;

/// Custom `SQLite` busy handler with exponential backoff and random jitter.
///
/// This handler is called whenever `SQLite` encounters a `SQLITE_BUSY` state
/// (e.g. when the database file is locked by another transaction).
///
/// Behavior:
/// - Uses exponential backoff: delays grow as 10, 20, 40, 80… ms.
/// - Adds a random jitter (0–(`JITTER_MS` - 1) ms) to reduce lock-step retries across
///   threads.
/// - Clamps the maximum delay per attempt to `MAX_DELAY_MS` ms.
/// - Always returns `1` to tell `SQLite` to retry, ensuring requests eventually complete.
///
/// Notes:
/// - The `n` parameter is the number of times the handler has been invoked for the same
///   lock.
/// - `_data` is a user-provided pointer passed via `sqlite3_busy_handler`, unused here.
extern "C" fn busy_handler(
    _data: *mut c_void,
    n: c_int,
) -> c_int {
    // add (`JITTER_MS` - 1) ms of randomness to avoid all clients retrying in lock-step
    let jitter = random::<u64>() % JITTER_MS;
    let exp: u32 = (n.saturating_sub(1)).try_into().unwrap_or(0);

    // grows exponentially: 10, 20, 40, 80… milliseconds
    let delay = 10u64.saturating_mul(2u64.saturating_pow(exp));

    // add the random shift, but ensure the total wait is ≤ `MAX_DELAY_MS` ms
    let wait_ms = delay.saturating_add(jitter).min(MAX_DELAY_MS);

    std::thread::sleep(Duration::from_millis(wait_ms));
    1
}

/// Configures the `SQLite` connection to use WAL mode with optimized settings for
/// concurrency.
///
/// This function performs several critical setup steps:
/// 1. Switches the journal mode to Write-Ahead Logging (WAL), which significantly
///    improves concurrency by allowing readers to proceed while a writer is active.
/// 2. Sets the synchronous level to NORMAL. This is a common performance optimization for
///    WAL mode, reducing disk syncs for non-critical moments, which is generally safe
///    against crashes but not power loss.
/// 3. Configures the WAL auto-checkpoint threshold. A checkpoint is the process of
///    transferring committed transactions from the WAL file back into the main database
///    file.
///
/// # Parameters
/// * `db_ptr`: A raw mutable pointer to an open `SQLite` database connection.
///
/// # Returns
/// * `Ok(())` on success.
/// * `Err(Errno)` if any of the configuration steps fail.
#[allow(unused)]
fn enable_wal_mode(db_ptr: *mut sqlite3) -> Result<(), Errno> {
    let pragmas = "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;";
    let c_pragmas = std::ffi::CString::new(pragmas).map_err(|_| Errno::ConvertingCString)?;

    let rc = unsafe {
        sqlite3_exec(
            db_ptr,
            c_pragmas.as_ptr(),
            None,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if rc != SQLITE_OK {
        return Err(Errno::Sqlite(rc));
    }

    // Set the WAL auto-checkpoint threshold to the default value of 1000 pages.
    // This means a checkpoint will be triggered automatically when the WAL file
    // grows to about 4MB (1000 pages * 4KB/page).
    let rc = unsafe { sqlite3_wal_autocheckpoint(db_ptr, 1000) };
    if rc != SQLITE_OK {
        return Err(Errno::Sqlite(rc));
    }

    Ok(())
}

/// Opens a connection to a new or existing `SQLite` database.
pub(super) fn open(
    readonly: bool,
    memory: bool,
    app_name: ApplicationName,
) -> Result<*mut sqlite3, Errno> {
    let mut db_ptr: *mut sqlite3 = std::ptr::null_mut();

    let (db_path, config) = if memory {
        let in_memory_config =
            get_app_in_memory_sqlite_db_cfg(app_name).ok_or(Errno::InvalidInMemoryConfig)?;

        (":memory:".into(), in_memory_config)
    } else {
        let persistent_config =
            get_app_persistent_sqlite_db_cfg(app_name).ok_or(Errno::InvalidPersistentConfig)?;

        let db_name = persistent_config
            .db_file
            .clone()
            .ok_or(Errno::MissingDatabaseNameForPersistentConfig)?;

        (db_name, persistent_config)
    };

    // Setting SQLITE_OPEN_NOMUTEX is enough, without setting env var, since
    // by default SQLITE_THREADSAFE=1 is used, and doc says:
    //
    // If single-thread mode has not been selected at compile-time or start-time,
    // then individual database connections can be created as either multi-thread or
    // serialized.
    let flags = if readonly {
        SQLITE_OPEN_READONLY
    } else {
        SQLITE_OPEN_CREATE | SQLITE_OPEN_READWRITE
    } | SQLITE_OPEN_NOMUTEX;

    let c_path =
        CString::new(db_path.to_string_lossy().as_bytes()).map_err(|_| Errno::ConvertingCString)?;

    let rc = unsafe { sqlite3_open_v2(c_path.as_ptr(), &raw mut db_ptr, flags, std::ptr::null()) };

    if rc != SQLITE_OK {
        return Err(Errno::Sqlite(rc));
    } else if db_ptr.is_null() {
        return Err(Errno::FailedOpeningDatabase);
    }

    let rc = unsafe { sqlite3_busy_handler(db_ptr, Some(busy_handler), std::ptr::null_mut()) };
    if rc != SQLITE_OK {
        return Err(Errno::Sqlite(rc));
    }

    if !memory && !readonly {
        enable_wal_mode(db_ptr)?;
    }

    // config database size limitation
    let rc = if memory {
        let size_limit = i64::from(config.max_db_size);

        unsafe { sqlite3_soft_heap_limit64(size_limit) };

        SQLITE_OK
    } else {
        let page_size = config.max_db_size / PAGE_SIZE;
        let pragma_stmt = format!("PRAGMA max_page_count = {page_size}");

        let c_pragma_stmt =
            std::ffi::CString::new(pragma_stmt).map_err(|_| Errno::ConvertingCString)?;

        unsafe {
            sqlite3_exec(
                db_ptr,
                c_pragma_stmt.as_ptr(),
                None,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        }
    };

    if rc != SQLITE_OK {
        return Err(Errno::FailedSettingDatabaseSize);
    }

    Ok(db_ptr)
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use std::{
        fs::{self, File},
        path::Path,
    };

    use serial_test::file_serial;

    use super::*;
    use crate::{app::ApplicationName, runtime_extensions::hermes::sqlite::connection::core};

    const TMP_DIR: &str = "tmp-dir";

    #[test]
    #[file_serial]
    fn test_open_success() {
        let app_name = ApplicationName(String::from(TMP_DIR));
        let config = get_app_persistent_sqlite_db_cfg(app_name.clone()).unwrap();
        let db_file = config.db_file.clone().unwrap();

        let db_ptr = open(false, false, app_name).unwrap();
        core::close(db_ptr).unwrap();

        let has_db_file = Path::new(&db_file).exists();
        let is_remove_success = fs::remove_file(Path::new(&db_file));

        assert!(has_db_file && is_remove_success.is_ok());
    }

    #[test]
    #[file_serial]
    fn test_open_readonly() {
        let app_name = ApplicationName(String::from(TMP_DIR));
        let config = get_app_persistent_sqlite_db_cfg(app_name.clone()).unwrap();
        let db_file = config.db_file.clone().unwrap();

        let file_result = File::create(&db_file);

        assert!(file_result.is_ok());

        let db_ptr = open(true, false, app_name).unwrap();

        let has_db_file = Path::new(&db_file).exists();
        let is_remove_success = fs::remove_file(Path::new(&db_file));

        core::close(db_ptr).unwrap();

        assert!(has_db_file && is_remove_success.is_ok());
    }

    #[test]
    #[file_serial]
    fn test_open_readonly_without_existing_file() {
        let app_name = ApplicationName(String::from(TMP_DIR));

        let db_ptr = open(true, false, app_name);

        assert!(db_ptr.is_err());
    }

    #[test]
    fn test_open_in_memory() {
        let app_name = ApplicationName(String::from(TMP_DIR));

        let db_ptr = open(false, true, app_name).unwrap();

        core::close(db_ptr).unwrap();
    }

    #[test]
    fn test_open_in_memory_readonly() {
        let app_name = ApplicationName(String::from(TMP_DIR));

        let db_ptr = open(true, true, app_name).unwrap();

        core::close(db_ptr).unwrap();
    }
}
