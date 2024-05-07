///! Hermes application configuration for modules.

/// Configuration struct for SQLite database.
///
/// This struct holds configuration options for SQLite database, including the path to the database
/// file and the maximum size of the database.

use crate::app::HermesAppName;

pub(crate) struct SqliteConfig {
  /// Path to the SQLite database file.
  db_file: Option<String>,
  /// Maximum size of the SQLite database in bytes.
  max_db_size: u32,
}

pub(crate) fn get_app_persistent_sqlite_db_cfg(_app_name: &HermesAppName) -> Option<SqliteConfig> {
  todo!()
}

pub(crate) fn get_app_inmemory_sqlite_db_cfg(_app_name: &HermesAppName) -> Option<SqliteConfig> {
  todo!()
}