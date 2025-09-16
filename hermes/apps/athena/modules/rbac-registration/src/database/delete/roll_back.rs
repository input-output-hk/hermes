//! Handle block rollback from volatile table.

use crate::{
    database::{operation::Operation, query_builder::QueryBuilder, statement::DatabaseStatement},
    hermes::sqlite::api::{Sqlite, Statement},
};

/// Prepare delete statement for deleting data when rollback happen from given volatile table.
pub(crate) fn prepare_roll_back_delete_from_volatile(
    sqlite: &Sqlite,
    table: &str,
) -> anyhow::Result<Statement> {
    const FUNCTION_NAME: &str = "prepare_roll_back_delete_from_volatile";
    DatabaseStatement::prepare_statement(
        sqlite,
        &QueryBuilder::delete_roll_forward(table),
        Operation::Delete,
        FUNCTION_NAME,
    )
    .map_err(|e| anyhow::anyhow!(e))
}

/// Delete data for handling rollback from volatile table.
pub(crate) fn roll_back_delete_from_volatile(
    stmt: &Statement,
    slot_no: u64,
) {
    const FUNCTION_NAME: &str = "roll_back_delete_from_volatile";
    if let Err(_) = DatabaseStatement::bind_slot(stmt, slot_no, FUNCTION_NAME) {
        return;
    }
}
