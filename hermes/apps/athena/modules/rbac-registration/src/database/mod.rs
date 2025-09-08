//! Database access layer for RBAC registration.

use once_cell::sync::{Lazy, OnceCell};
use serde_json::json;
pub(crate) mod create;
pub(crate) mod data;
pub(crate) mod insert;
pub(crate) mod select;

use crate::{
    hermes::hermes::{
        self,
        sqlite::api::{open, Sqlite, Statement, Value},
    },
    utils::log::{log_error, log_info},
};
use std::sync::LazyLock;

/// Open database connection.
pub(crate) fn open_db_connection() -> Result<Sqlite, ()> {
    const FUNCTION_NAME: &str = "open_db_connection";

    match open(false, false) {
        Ok(db) => Ok(db),
        Err(e) => {
            log_error(
                file!(),
                FUNCTION_NAME,
                "hermes::sqlite::api::open",
                &format!("Failed to open database: {e}"),
                None,
            );
            Err(())
        },
    }
}

pub(crate) fn close_db_connection(sqlite: Sqlite) {
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
pub(crate) fn bind_with_log(
    stmt: &Statement,
    func_name: &str,
    idx: u32,
    value: &Value,
    field_name: &str,
) -> anyhow::Result<()> {
    if let Err(e) = stmt.bind(idx, value) {
        log_error(
            file!(),
            func_name,
            "hermes::sqlite::bind",
            &format!("Failed to bind: {e:?}"),
            Some(&json!({ field_name: format!("{value:?}") }).to_string()),
        );
        anyhow::bail!("Failed to bind {field_name}");
    }
    Ok(())
}
