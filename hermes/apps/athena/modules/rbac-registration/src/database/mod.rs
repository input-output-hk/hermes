//! Database access layer for RBAC registration.

use once_cell::sync::{Lazy, OnceCell};
pub(crate) mod create;
pub(crate) mod data;
pub(crate) mod insert;
pub(crate) mod select;

use crate::{
    hermes::{
        self,
        hermes::sqlite::api::{Sqlite, Statement, Value},
    },
    utils::log::{log_error, log_info},
};
use std::sync::LazyLock;

const FILE_NAME: &str = "rbac-registration/src/database/mod.rs";

pub static SQLITE: LazyLock<hermes::hermes::sqlite::api::Sqlite> = LazyLock::new(|| {
    log_info(FILE_NAME, "", "", &format!("Open db 🍔"), None);
    match hermes::hermes::sqlite::api::open(false, false) {
        Ok(db) => db,
        Err(e) => {
            log_error(
                FILE_NAME,
                "lazy open sqlite",
                "hermes::hermes::sqlite::api::open",
                &format!("Failed to open database: {e}"),
                None,
            );
            panic!("Failed to open database: {e}",);
        },
    }
});
