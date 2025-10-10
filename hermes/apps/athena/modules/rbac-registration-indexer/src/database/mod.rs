//! Database access layer for RBAC registration.

pub(crate) mod create;
pub(crate) mod data;
pub(crate) mod delete;
pub(crate) mod insert;
pub(crate) mod query_builder;

/// RBAC registration persistent table name.
pub(crate) const RBAC_REGISTRATION_PERSISTENT_TABLE_NAME: &str = "rbac_registration_persistent";
/// RBAC registration volatile table name.
pub(crate) const RBAC_REGISTRATION_VOLATILE_TABLE_NAME: &str = "rbac_registration_volatile";
/// RBAC stake address persistent table name.
pub(crate) const RBAC_STAKE_ADDRESS_PERSISTENT_TABLE_NAME: &str = "rbac_stake_address_persistent";
/// RBAC stake address volatile table name.
pub(crate) const RBAC_STAKE_ADDRESS_VOLATILE_TABLE_NAME: &str = "rbac_stake_address_volatile";
