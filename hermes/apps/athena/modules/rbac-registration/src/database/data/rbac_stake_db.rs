//! Data for the `rbac_stake_address` table.

/// Data of the `rbac_stake_address` table.
#[derive(Debug, Clone)]
pub(crate) struct RbacStakeDbData {
    /// 29 bytes stake address - CIP19.
    pub(crate) stake_address: Vec<u8>,
    /// Slot number.
    pub(crate) slot: u64,
    /// Transaction index.
    pub(crate) txn_idx: u16,
    /// Optional Catalyst short ID - this only exist for Role 0.
    pub(crate) catalyst_id: Option<String>,
    /// 32 bytes transaction ID (aka transaction hash).
    pub(crate) txn_id: Vec<u8>,
}
