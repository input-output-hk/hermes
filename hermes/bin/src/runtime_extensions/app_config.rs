//! Hermes application configuration for modules.

use std::path::PathBuf;

use crate::app::HermesAppName;

/// Configuration struct for `SQLite` database.
///
/// This struct holds configuration options for `SQLite` database, including the path to
/// the database file and the maximum size of the database.

const MAX_CONFIG_DB_SIZE: u32 = 1_048_576;

/// Represents config object for `SQLite`
pub(crate) struct SqliteConfig {
    /// Path to the `SQLite` database file, not set if it's in-memory database.
    pub(crate) db_file: Option<PathBuf>,
    /// Maximum size of the `SQLite` database in bytes.
    pub(crate) max_db_size: u32,
}

/// Gets `SQLite` config for persistent datastore
pub(crate) fn get_app_persistent_sqlite_db_cfg(app_name: HermesAppName) -> Option<SqliteConfig> {
    let HermesAppName(name) = app_name;

    if name.is_empty() {
        return None;
    }

    Some(SqliteConfig {
        db_file: Some(PathBuf::from("hermes_datastore.db")),
        max_db_size: MAX_CONFIG_DB_SIZE,
    })
}

/// Gets `SQLite` config for in-memory datastore
pub(crate) fn get_app_in_memory_sqlite_db_cfg(app_name: HermesAppName) -> Option<SqliteConfig> {
    let HermesAppName(name) = app_name;

    if name.is_empty() {
        return None;
    }

    Some(SqliteConfig {
        db_file: None,
        max_db_size: MAX_CONFIG_DB_SIZE,
    })
}
