//! Database access layer for RBAC registration.

pub(crate) mod query_builder;
pub(crate) mod select;

/// RBAC registration persistent table name.
pub(crate) const RBAC_REGISTRATION_PERSISTENT_TABLE_NAME: &str = "rbac_registration_persistent";
/// RBAC registration volatile table name.
pub(crate) const RBAC_REGISTRATION_VOLATILE_TABLE_NAME: &str = "rbac_registration_volatile";
/// RBAC stake address persistent table name.
pub(crate) const RBAC_STAKE_ADDRESS_PERSISTENT_TABLE_NAME: &str = "rbac_stake_address_persistent";
/// RBAC stake address volatile table name.
pub(crate) const RBAC_STAKE_ADDRESS_VOLATILE_TABLE_NAME: &str = "rbac_stake_address_volatile";
