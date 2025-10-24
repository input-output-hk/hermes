//! Database access layer for RBAC registration.
// TODO - This is redundant to what is in rbac-registration module, need to move the share

pub(crate) mod query_builder;
pub(crate) mod select;

/// RBAC registration persistent table name.
pub(crate) const RBAC_REGISTRATION_PERSISTENT_TABLE_NAME: &str = "rbac_registration_persistent";
/// RBAC registration volatile table name.
pub(crate) const RBAC_REGISTRATION_VOLATILE_TABLE_NAME: &str = "rbac_registration_volatile";
