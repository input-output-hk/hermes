//! Core functionality implementation for the `SQLite` open function.

use std::ffi::CString;

use libsqlite3_sys::{
    sqlite3, sqlite3_exec, sqlite3_open_v2, sqlite3_soft_heap_limit64, SQLITE_OK,
    SQLITE_OPEN_CREATE, SQLITE_OPEN_READONLY, SQLITE_OPEN_READWRITE,
};

use crate::{
    app::ApplicationName,
    runtime_extensions::{
        app_config::{get_app_in_memory_sqlite_db_cfg, get_app_persistent_sqlite_db_cfg},
        bindings::hermes::sqlite::api::Errno,
    },
};

/// The default page size of `SQLite`.
const PAGE_SIZE: u32 = 4_096;

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
    let flags = if readonly {
        SQLITE_OPEN_READONLY
    } else {
        SQLITE_OPEN_CREATE | SQLITE_OPEN_READWRITE
    };

    let c_path =
        CString::new(db_path.to_string_lossy().as_bytes()).map_err(|_| Errno::ConvertingCString)?;

    let rc = unsafe { sqlite3_open_v2(c_path.as_ptr(), &raw mut db_ptr, flags, std::ptr::null()) };

    if rc != SQLITE_OK {
        return Err(Errno::Sqlite(rc));
    } else if db_ptr.is_null() {
        return Err(Errno::FailedOpeningDatabase);
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

#[cfg(test)]
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
