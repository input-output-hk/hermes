//! Insert data to `rbac_registration` table.

use shared::{
    bindings::hermes::sqlite::api::{Sqlite, Statement, Value},
    sqlite_bind_parameters,
    utils::{
        log::log_error,
        sqlite::{operation::Operation, statement::DatabaseStatement},
    },
};

use crate::database::{data::rbac_db::RbacDbData, query_builder::QueryBuilder};

/// Prepare insert statement for `rbac_registration` table.
pub(crate) fn prepare_insert_rbac_registration(
    sqlite: &Sqlite,
    table: &str,
) -> anyhow::Result<Statement> {
    const FUNCTION_NAME: &str = "prepare_insert_rbac_registration";
    DatabaseStatement::prepare_statement(
        sqlite,
        &QueryBuilder::insert_rbac_registration(table),
        Operation::Insert,
        FUNCTION_NAME,
    )
    .map_err(|e| anyhow::anyhow!(e))
}

/// Insert data to `rbac_registration` table.
pub(crate) fn insert_rbac_registration(
    stmt: &Statement,
    data: RbacDbData,
) {
    const FUNCTION_NAME: &str = "insert_rbac_registration";
    drop(DatabaseStatement::bind_step_reset_statement(
        stmt,
        |stmt| bind_rbac_registration(stmt, data),
        FUNCTION_NAME,
    ));
}

/// Bind data to `rbac_registration` table.
fn bind_rbac_registration(
    stmt: &Statement,
    data: RbacDbData,
) -> anyhow::Result<()> {
    const FUNCTION_NAME: &str = "bind_rbac_registration";

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
        data.txn_id => "txn_id",
        slot => "slot_no",
        data.txn_idx => "txn_idx",
        data.prv_txn_id => "prv_txn_id",
        data.purpose => "purpose",
        data.catalyst_id.map(|id| id.trim().to_string()) => "catalyst_id",
        data.problem_report => "problem_report"
    )?;
    Ok(())
}
