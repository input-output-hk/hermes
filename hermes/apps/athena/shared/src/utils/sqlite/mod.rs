//! Database utilities.

pub mod operation;
pub mod statement;
pub mod value;

use std::ops::Deref;

use log::error;

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

/// Sqlite connection. Closes on drop.
pub struct Connection(Sqlite);

impl Deref for Connection {
    type Target = Sqlite;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Connection {
    /// Open a writable sqlite connection.
    pub fn open(in_memory: bool) -> anyhow::Result<Self> {
        open(false, in_memory)
            .map(Self)
            .inspect_err(|error| error!(error:%, in_memory; "Failed to open database"))
            .map_err(anyhow::Error::from)
    }

    /// Close the connection explicitly.
    pub fn close(&self) -> anyhow::Result<()> {
        self.0
            .close()
            .inspect_err(|error| error!(error:%; "Failed to close database"))
            .map_err(anyhow::Error::from)
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.close();
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
