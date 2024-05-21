///! Core functionality implementation for the `SQLite` open function.
use libsqlite3_sys::{
    sqlite3, sqlite3_exec, sqlite3_file_control, sqlite3_open_v2, SQLITE_ERROR,
    SQLITE_FCNTL_SIZE_LIMIT, SQLITE_OK, SQLITE_OPEN_CREATE, SQLITE_OPEN_READONLY,
    SQLITE_OPEN_READWRITE,
};

use crate::{
    app::HermesAppName,
    runtime_extensions::{
        app_config::{get_app_inmemory_sqlite_db_cfg, get_app_persistent_sqlite_db_cfg},
        bindings::hermes::sqlite::api::Errno,
    },
};

/// Represents the various errors that can occur when opening a database.
pub(super) enum OpenError {
    /// The in-memory configuration provided is invalid.
    InvalidInMemoryConfig,
    /// The persistent configuration provided is invalid.
    InvalidPersistentConfig,
    /// The database name is missing in the persistent configuration.
    MissingDatabaseNameForPersistentConfig,
    /// Failed to open the database.
    FailedOpeningDatabase,
    /// Failed to set the database size.
    FailedSettingDatabaseSize,
    /// An error occurred with `SQLite`, represented by an `Errno`.
    SQLiteError(Errno),
}

/// Opens a connection to a new or existing `SQLite` database.
pub(super) fn open(
    readonly: bool, memory: bool, app_name: HermesAppName,
) -> Result<*mut sqlite3, OpenError> {
    let mut db_ptr: *mut sqlite3 = std::ptr::null_mut();

    let (db_path, config) = if memory {
        let inmemory_config = match get_app_inmemory_sqlite_db_cfg(app_name) {
            Some(config) => config,
            None => return Err(OpenError::InvalidInMemoryConfig),
        };

        (":memory:".into(), inmemory_config)
    } else {
        let persistent_config = match get_app_persistent_sqlite_db_cfg(app_name) {
            Some(config) => config,
            None => return Err(OpenError::InvalidPersistentConfig),
        };

        let db_name = match &persistent_config.db_file {
            Some(db_name) => db_name.clone(),
            None => return Err(OpenError::MissingDatabaseNameForPersistentConfig),
        };

        (db_name, persistent_config)
    };
    let flags = if readonly {
        SQLITE_OPEN_READONLY
    } else {
        SQLITE_OPEN_CREATE | SQLITE_OPEN_READWRITE
    };

    let result = unsafe {
        sqlite3_open_v2(
            db_path.as_str().as_ptr().cast(),
            &mut db_ptr,
            flags,
            std::ptr::null(),
        )
    };

    if result != SQLITE_OK {
        return Err(OpenError::SQLiteError(result.into()));
    } else if db_ptr.is_null() {
        return Err(OpenError::FailedOpeningDatabase);
    }

    // config database size limitation
    let rc = if memory {
        let size_limit = i64::from(config.max_db_size);

        unsafe {
            sqlite3_file_control(
                db_ptr,
                "main\0".as_ptr().cast::<i8>(),
                SQLITE_FCNTL_SIZE_LIMIT,
                size_limit as *mut std::ffi::c_void,
            )
        }
    } else {
        let page_size = config.max_db_size / 4_096;
        let pragma_stmt = format!("PRAGMA max_page_count = {page_size}");

        let c_pragma_stmt = match std::ffi::CString::new(pragma_stmt) {
            Ok(value) => value,
            Err(_) => return Err(OpenError::SQLiteError(SQLITE_ERROR.into())),
        };

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
        return Err(OpenError::FailedSettingDatabaseSize);
    }

    Ok(db_ptr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::HermesAppName;

    #[test]
    fn test_open_success() {
        let app_name = HermesAppName(String::from("tmp"));

        let db_ptr = open(false, false, app_name);

        assert!(db_ptr.is_ok());
    }
}
