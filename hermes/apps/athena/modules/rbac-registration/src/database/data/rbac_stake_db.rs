#[derive(Debug, Clone)]
pub(crate) struct RbacStakeDbData {
    pub(crate) stake_address: Vec<u8>,
    pub(crate) slot: u64,
    pub(crate) txn_idx: u16,
    pub(crate) catalyst_id: Option<String>,
}

impl RbacStakeDbData {
    fn new(
        stake_address: Vec<u8>,
        slot: u64,
        txn_idx: u16,
        catalyst_id: Option<String>,
    ) -> Self {
        Self {
            stake_address,
            slot,
            txn_idx,
            catalyst_id,
        }
    }
}
