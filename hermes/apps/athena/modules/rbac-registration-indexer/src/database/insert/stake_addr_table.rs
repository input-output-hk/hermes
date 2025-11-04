//! Insert data to `rbac_stake_address` table.

use shared::{
    bindings::hermes::sqlite::api::{Sqlite, Statement, Value},
    sqlite_bind_parameters,
    utils::{
        log::log_error,
        sqlite::{operation::Operation, statement::DatabaseStatement},
    },
};

use crate::database::{data::rbac_stake_db::RbacStakeDbData, query_builder::QueryBuilder};

/// Prepare insert statement for `rbac_stake_address` table.
pub(crate) fn prepare_insert_rbac_stake_address(
    sqlite: &Sqlite,
    table: &str,
) -> anyhow::Result<Statement> {
    const FUNCTION_NAME: &str = "prepare_insert_rbac_stake_address";
    DatabaseStatement::prepare_statement(
        sqlite,
        &QueryBuilder::insert_rbac_stake_address(table),
        Operation::Insert,
        FUNCTION_NAME,
    )
    .map_err(|e| anyhow::anyhow!(e))
}

/// Insert data to `rbac_stake_address` table.
pub(crate) fn insert_rbac_stake_address(
    stmt: &Statement,
    data: RbacStakeDbData,
) {
    const FUNCTION_NAME: &str = "insert_rbac_stake_address";
    drop(DatabaseStatement::bind_step_reset_statement(
        stmt,
        |stmt| bind_rbac_stake_address(stmt, data),
        FUNCTION_NAME,
    ));
}

/// Bind data to `rbac_stake_address` table.
fn bind_rbac_stake_address(
    stmt: &Statement,
    data: RbacStakeDbData,
) -> anyhow::Result<()> {
    const FUNCTION_NAME: &str = "bind_rbac_stake_address";

    let slot: Value = match data.slot.try_into() {
        Ok(s) => s,
        Err(e) => {
            log_error(
                file!(),
                FUNCTION_NAME,
                "slot.try_into()",
                &format!("Failed to convert slot: {e}"),
                None,
            );
            anyhow::bail!("Failed to convert slot: {e}");
        },
    };
    sqlite_bind_parameters!(stmt, FUNCTION_NAME,
        data.stake_address => "stake_address",
        slot => "slot_no",
        data.txn_idx => "txn_idx",
        data.catalyst_id.map(|id| id.trim().to_string()) => "catalyst_id",
        data.txn_id => "txn_id"
    )?;
    Ok(())
}
