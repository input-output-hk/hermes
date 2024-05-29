use crate::app::HermesAppName;

/// ! Hermes application configuration for modules.

/// Configuration struct for `SQLite` database.
///
/// This struct holds configuration options for `SQLite` database, including the path to
/// the database file and the maximum size of the database.

const MAX_CONFIG_DB_SIZE: u32 = 1_048_576;

pub(crate) struct SqliteConfig {
    /// Path to the `SQLite` database file, not set if it's in-memory database.
    pub(crate) db_file: Option<String>,
    /// Maximum size of the `SQLite` database in bytes.
    pub(crate) max_db_size: u32,
}

pub(crate) fn get_app_persistent_sqlite_db_cfg(_app_name: HermesAppName) -> Option<SqliteConfig> {
    Some(SqliteConfig {
        db_file: Some(String::from("hermes_datastore.db")),
        max_db_size: MAX_CONFIG_DB_SIZE,
    })
}

pub(crate) fn get_app_in_memory_sqlite_db_cfg(_app_name: HermesAppName) -> Option<SqliteConfig> {
    Some(SqliteConfig {
        db_file: None,
        max_db_size: MAX_CONFIG_DB_SIZE,
    })
}
