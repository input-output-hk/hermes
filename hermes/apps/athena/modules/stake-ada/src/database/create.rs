//! Create the database tables for RBAC registration.

use shared::{
    bindings::hermes::sqlite::api::Sqlite,
    utils::sqlite::{operation::Operation, statement::DatabaseStatement},
};

use crate::database::{
    query_builder::QueryBuilder, STAKE_REGISTRATION_TABLE_PERSISTENT_TABLE_NAME,
    STAKE_REGISTRATION_TABLE_VOLATILE_TABLE_NAME, TXI_BY_TXN_ID_PERSISTENT_TABLE_NAME,
    TXI_BY_TXN_ID_VOLATILE_TABLE_NAME, TXO_BY_STAKE_ADDRESS_PERSISTENT_TABLE_NAME,
    TXO_BY_STAKE_ADDRESS_VOLATILE_TABLE_NAME,
};

/// Create a staked ada tables.
pub(crate) fn create_staked_ada_persistent_tables(sqlite: &Sqlite) -> anyhow::Result<()> {
    const FUNCTION_NAME: &str = "create_staked_ada_persistent_tables";
    DatabaseStatement::execute_statement(
        sqlite,
        &QueryBuilder::create_stake_registration_table(
            STAKE_REGISTRATION_TABLE_PERSISTENT_TABLE_NAME,
        ),
        Operation::Create,
        FUNCTION_NAME,
    )?;
    DatabaseStatement::execute_statement(
        sqlite,
        &QueryBuilder::create_txi_by_txn_id_table(TXI_BY_TXN_ID_PERSISTENT_TABLE_NAME),
        Operation::Create,
        FUNCTION_NAME,
    )?;
    DatabaseStatement::execute_statement(
        sqlite,
        &QueryBuilder::create_txi_by_txn_id_table(TXO_BY_STAKE_ADDRESS_PERSISTENT_TABLE_NAME),
        Operation::Create,
        FUNCTION_NAME,
    )
}

/// Create a staked ada tables.
pub(crate) fn create_staked_ada_volatile_tables(sqlite: &Sqlite) -> anyhow::Result<()> {
    const FUNCTION_NAME: &str = "create_staked_ada_persistent_tables";
    DatabaseStatement::execute_statement(
        sqlite,
        &QueryBuilder::create_stake_registration_table(
            STAKE_REGISTRATION_TABLE_VOLATILE_TABLE_NAME,
        ),
        Operation::Create,
        FUNCTION_NAME,
    )?;
    DatabaseStatement::execute_statement(
        sqlite,
        &QueryBuilder::create_txi_by_txn_id_table(TXI_BY_TXN_ID_VOLATILE_TABLE_NAME),
        Operation::Create,
        FUNCTION_NAME,
    )?;
    DatabaseStatement::execute_statement(
        sqlite,
        &QueryBuilder::create_txi_by_txn_id_table(TXO_BY_STAKE_ADDRESS_VOLATILE_TABLE_NAME),
        Operation::Create,
        FUNCTION_NAME,
    )
}
