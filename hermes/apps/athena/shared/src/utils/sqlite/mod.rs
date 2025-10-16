//! Database utilities.

mod wrappers;

pub mod operation;
pub mod statement;
pub mod value;

pub use wrappers::{Connection, Row, Rows, Statement};

pub use crate::bindings::hermes::sqlite::api::Value;
use crate::{
    bindings::hermes::sqlite::api::{open, Sqlite},
    utils::log::log_error,
};

/// Open database connection.
pub fn open_db_connection(is_mem: bool) -> anyhow::Result<Sqlite> {
    const FUNCTION_NAME: &str = "open_db_connection";
    match open(false, is_mem) {
        Ok(db) => Ok(db),
        Err(e) => {
            let error = "Failed to open database";
            log_error(
                file!(),
                FUNCTION_NAME,
                "hermes::sqlite::api::open",
                &format!("{error}: {e}"),
                None,
            );
            anyhow::bail!(error)
        },
    }
}

/// Close database connection.
pub fn close_db_connection(sqlite: Sqlite) {
    const FUNCTION_NAME: &str = "close_db_connection";
    if let Err(e) = sqlite.close() {
        log_error(
            file!(),
            FUNCTION_NAME,
            "hermes::sqlite::api::close",
            &format!("Failed to close database: {e}"),
            None,
        );
    }
}

// --------------- Binding helper -------------------

/// A macro to bind parameters to a prepared statement.
#[macro_export]
macro_rules! sqlite_bind_parameters {
    ($stmt:expr, $func_name:expr, $($field:expr => $field_name:expr),+) => {
        {
            let mut idx = 0;
            $(
                idx += 1;
                let value: Value = $field.try_into()?;
                if let Err(e) = $stmt.bind(idx, &value) {
                   log_error(
                        file!(),
                        $func_name,
                        "hermes::sqlite::bind",
                        &format!("Failed to bind: {e:?}"),
                        Some(&serde_json::json!({ $field_name: format!("{value:?}") }).to_string()),
                    );
                    anyhow::bail!("Failed to bind {}", $field_name);
                }
            )*
            Ok::<(), anyhow::Error>(())
        }
    };
}
