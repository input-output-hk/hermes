//! Create the database tables for RBAC registration.

use crate::{
    database::{
        operation::Operation, query_builder::QueryBuilder, statement::DatabaseStatement,
        RBAC_REGISTRATION_PERSISTENT_TABLE_NAME, RBAC_REGISTRATION_VOLATILE_TABLE_NAME,
        RBAC_STAKE_ADDRESS_PERSISTENT_TABLE_NAME, RBAC_STAKE_ADDRESS_VOLATILE_TABLE_NAME,
    },
    hermes::sqlite::api::Sqlite,
};

/// Create a persistent `rbac_registration` and `rbac_stake_address` table.
pub(crate) fn create_rbac_persistent_tables(sqlite: &Sqlite) {
    const FUNCTION_NAME: &str = "create_rbac_persistent_tables";

    if let Err(_) = DatabaseStatement::execute_statement(
        sqlite,
        &QueryBuilder::create_rbac_registration_table(RBAC_REGISTRATION_PERSISTENT_TABLE_NAME),
        Operation::CREATE,
        FUNCTION_NAME,
    ) {
        return;
    }

    if let Err(_) = DatabaseStatement::execute_statement(
        sqlite,
        &QueryBuilder::create_rbac_stake_address_table(RBAC_STAKE_ADDRESS_PERSISTENT_TABLE_NAME),
        Operation::CREATE,
        FUNCTION_NAME,
    ) {
        return;
    }
}

/// Create a volatile `rbac_registration` and `rbac_stake_address` table.
pub(crate) fn create_rbac_volatile_tables(sqlite: &Sqlite) {
    const FUNCTION_NAME: &str = "create_rbac_volatile_tables";

    if let Err(_) = DatabaseStatement::execute_statement(
        sqlite,
        &QueryBuilder::create_rbac_registration_table(RBAC_REGISTRATION_VOLATILE_TABLE_NAME),
        Operation::CREATE,
        FUNCTION_NAME,
    ) {
        return;
    }
    if let Err(_) = DatabaseStatement::execute_statement(
        sqlite,
        &QueryBuilder::create_rbac_stake_address_table(RBAC_STAKE_ADDRESS_VOLATILE_TABLE_NAME),
        Operation::CREATE,
        FUNCTION_NAME,
    ) {
        return;
    }
}
