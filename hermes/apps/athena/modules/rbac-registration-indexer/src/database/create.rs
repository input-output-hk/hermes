//! Create the database tables for RBAC registration.

use shared::{
    bindings::hermes::sqlite::api::Sqlite,
    utils::sqlite::{operation::Operation, statement::DatabaseStatement},
};

use crate::database::{
    RBAC_REGISTRATION_PERSISTENT_TABLE_NAME, RBAC_REGISTRATION_VOLATILE_TABLE_NAME,
    RBAC_STAKE_ADDRESS_PERSISTENT_TABLE_NAME, RBAC_STAKE_ADDRESS_VOLATILE_TABLE_NAME,
    query_builder::QueryBuilder,
};

/// Create a persistent `rbac_registration` and `rbac_stake_address` table.
pub(crate) fn create_rbac_persistent_tables(sqlite: &Sqlite) {
    const FUNCTION_NAME: &str = "create_rbac_persistent_tables";

    if DatabaseStatement::execute_statement(
        sqlite,
        &QueryBuilder::create_rbac_registration_table(RBAC_REGISTRATION_PERSISTENT_TABLE_NAME),
        Operation::Create,
        FUNCTION_NAME,
    )
    .is_err()
    {
        return;
    }

    if DatabaseStatement::execute_statement(
        sqlite,
        &QueryBuilder::create_rbac_stake_address_table(RBAC_STAKE_ADDRESS_PERSISTENT_TABLE_NAME),
        Operation::Create,
        FUNCTION_NAME,
    )
    .is_err()
    {}
}

/// Create a volatile `rbac_registration` and `rbac_stake_address` table.
pub(crate) fn create_rbac_volatile_tables(sqlite: &Sqlite) {
    const FUNCTION_NAME: &str = "create_rbac_volatile_tables";

    if DatabaseStatement::execute_statement(
        sqlite,
        &QueryBuilder::create_rbac_registration_table(RBAC_REGISTRATION_VOLATILE_TABLE_NAME),
        Operation::Create,
        FUNCTION_NAME,
    )
    .is_err()
    {
        return;
    }
    if DatabaseStatement::execute_statement(
        sqlite,
        &QueryBuilder::create_rbac_stake_address_table(RBAC_STAKE_ADDRESS_VOLATILE_TABLE_NAME),
        Operation::Create,
        FUNCTION_NAME,
    )
    .is_err()
    {}
}
