pub(crate) mod create;
pub(crate) mod query_builder;

/// STAKE_REGISTRATION persistent table name.
pub(crate) const STAKE_REGISTRATION_TABLE_PERSISTENT_TABLE_NAME: &str = "stake_registration";
/// STAKE_REGISTRATION volatile table name.
pub(crate) const STAKE_REGISTRATION_TABLE_VOLATILE_TABLE_NAME: &str = "stake_registration";
/// TXI_BY_TXN_ID persistent table name.
pub(crate) const TXI_BY_TXN_ID_PERSISTENT_TABLE_NAME: &str = "txi_by_txn_id";
/// TXI_BY_TXN_ID volatile table name.
pub(crate) const TXI_BY_TXN_ID_VOLATILE_TABLE_NAME: &str = "txi_by_txn_id";
/// TXO_BY_STAKE_ADDRESS persistent table name.
pub(crate) const TXO_BY_STAKE_ADDRESS_PERSISTENT_TABLE_NAME: &str = "txo_by_stake_address";
/// TXO_BY_STAKE_ADDRESS volatile table name.
pub(crate) const TXO_BY_STAKE_ADDRESS_VOLATILE_TABLE_NAME: &str = "txo_by_stake_address";
