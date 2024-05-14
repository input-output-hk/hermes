///! Hermes application configuration for modules.

/// Configuration struct for SQLite database.
///
/// This struct holds configuration options for SQLite database, including the path to the database
/// file and the maximum size of the database.

use crate::app::HermesAppName;

pub(crate) struct SqliteConfig {
  /// Path to the SQLite database file, not set if it's in-memory database.
  pub(crate) db_file: Option<String>,
  /// Maximum size of the SQLite database in bytes.
  pub(crate) max_db_size: u32,
}

pub(crate) fn get_app_persistent_sqlite_db_cfg() -> Option<SqliteConfig> {
  Some(SqliteConfig {
    db_file: Some(String::from("hermes_datastore.db")),
    max_db_size: 1_048_576
  })
}

pub(crate) fn get_app_inmemory_sqlite_db_cfg() -> Option<SqliteConfig> {
  Some(SqliteConfig {
    db_file: None,
    max_db_size: 1_048_576
  })
}