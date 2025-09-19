//! Select from the database.

pub(crate) mod cat_id;
pub(crate) mod stake_addr;

use crate::{
    hermes::sqlite::api::{Statement, Value},
    utils::log::log_error,
};

/// Convert a SQLite column value to a Rust type.
pub(crate) fn column_as<T>(
    stmt: &Statement,
    index: u32,
    func_name: &str,
    field_name: &str,
) -> anyhow::Result<T>
where
    T: TryFrom<Value, Error = anyhow::Error>,
{
    let value = stmt.column(index)?;
    T::try_from(value).map_err(|e| {
        log_error(
            file!(),
            func_name,
            "column_as",
            &format!(
                "Failed to convert column {} to {}: {}",
                field_name,
                std::any::type_name::<T>(),
                e
            ),
            None,
        );
        e
    })
}
