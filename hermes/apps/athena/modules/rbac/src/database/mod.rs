pub(crate) mod operation;
pub(crate) mod query_builder;
pub(crate) mod select;
pub(crate) mod statement;

/// RBAC registration persistent table name.
pub(crate) const RBAC_REGISTRATION_PERSISTENT_TABLE_NAME: &str = "rbac_registration_persistent";
/// RBAC registration volatile table name.
pub(crate) const RBAC_REGISTRATION_VOLATILE_TABLE_NAME: &str = "rbac_registration_volatile";
/// RBAC stake address persistent table name.
pub(crate) const RBAC_STAKE_ADDRESS_PERSISTENT_TABLE_NAME: &str = "rbac_stake_address_persistent";
/// RBAC stake address volatile table name.
pub(crate) const RBAC_STAKE_ADDRESS_VOLATILE_TABLE_NAME: &str = "rbac_stake_address_volatile";

/// Open database connection.
pub(crate) fn open_db_connection(is_mem: bool) -> anyhow::Result<Sqlite> {
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

/// A macro to bind parameters to a prepared statement.
#[macro_export]
macro_rules! bind_parameters {
    ($stmt:expr, $func_name:expr, $($field:expr => $field_name:expr),*) => {
        {
            let mut idx = 1;
            $(
                let value: Value = $field.into();
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
                idx += 1;
            )*
            Ok::<(), anyhow::Error>(())
        }
    };
}
