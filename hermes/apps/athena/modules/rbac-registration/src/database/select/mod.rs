//! RBAC registration database access select.
//! Information of how things work can be viewed in
//! <https://github.com/input-output-hk/catalyst-libs/blob/main/rust/rbac-registration/examples.md>

use crate::{
    hermes::hermes::sqlite::api::{Statement, Value},
    utils::log::{log_error, log_select_column},
};

pub(crate) mod select_cat_id;
pub(crate) mod select_stake;

/// Helper function to get column value.
pub(crate) fn column_as<T>(
    stmt: &Statement,
    idx: u32,
    func_name: &str,
    field: &str,
) -> anyhow::Result<T>
where
    T: TryFrom<Value, Error = anyhow::Error>,
{
    let value = stmt.column(idx).map_err(|_| {
        log_select_column(file!(), func_name, idx, field);
        anyhow::anyhow!("Failed to get column {} at index {}", field, idx)
    })?;

    let v = value.try_into().map_err(|e| {
        log_error(
            file!(),
            func_name,
            "Value::try_from",
            &format!("Failed to convert {field}: {e}"),
            None,
        );
        e
    })?;

    Ok(v)
}
