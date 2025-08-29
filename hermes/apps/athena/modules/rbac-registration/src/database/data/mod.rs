use crate::hermes::hermes::sqlite::api::Value;
// FIXME: cleanup

#[derive(Debug, Clone)]
pub(crate) struct RbacDbData {
    pub(crate) txn_id: Vec<u8>,
    pub(crate) catalyst_id: Option<String>,
    pub(crate) slot: u64,
    pub(crate) txn_idx: u16,
    pub(crate) prv_txn_id: Option<Vec<u8>>,
    pub(crate) purpose: Option<String>,
    pub(crate) problem_report: Option<String>,
}

impl RbacDbData {
    fn new(
        txn_id: Vec<u8>,
        catalyst_id: Option<String>,
        slot: u64,
        txn_idx: u16,
        prv_txn_id: Option<Vec<u8>>,
        purpose: Option<String>,
        problem_report: Option<String>,
    ) -> Self {
        Self {
            txn_id,
            catalyst_id,
            slot,
            txn_idx,
            prv_txn_id,
            purpose,
            problem_report,
        }
    }
}

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
