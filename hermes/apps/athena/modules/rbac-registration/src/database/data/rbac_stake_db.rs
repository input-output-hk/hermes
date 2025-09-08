//! Data for the `rbac_stake_address` table.

/// Data of the `rbac_stake_address` table.
#[derive(Debug, Clone)]
pub(crate) struct RbacStakeDbData {
    /// 29 bytes stake address.
    pub(crate) stake_address: Vec<u8>,
    /// Slot number.
    pub(crate) slot: u64,
    /// Transaction index.
    pub(crate) txn_idx: u16,
    /// Optional Catalyst short ID.
    pub(crate) catalyst_id: Option<String>,
    /// 32 bytes transaction ID (aka transaction hash).
    pub(crate) txn_id: Vec<u8>,
}

impl RbacStakeDbData {
    /// Create new instance of the `RbacStakeDbData`
    fn new(
        stake_address: Vec<u8>,
        slot: u64,
        txn_idx: u16,
        catalyst_id: Option<String>,
        txn_id: Vec<u8>,
    ) -> Self {
        Self {
            stake_address,
            slot,
            txn_idx,
            catalyst_id,
            txn_id,
        }
    }
}
