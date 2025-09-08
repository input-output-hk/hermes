//! Data for the `rbac_registration` table.

#[derive(Debug, Clone)]
/// Data of the `rbac_registration` table.
pub(crate) struct RbacDbData {
    /// 32 bytes transaction ID (aka transaction hash).
    pub(crate) txn_id: Vec<u8>,
    /// Optional Catalyst short ID.
    pub(crate) catalyst_id: Option<String>,
    /// Slot number
    pub(crate) slot: u64,
    /// Transaction index.
    pub(crate) txn_idx: u16,
    /// Optional previous transaction ID (aka transaction hash).
    /// Used to link to the previous RBAC registration.
    /// If None, it can indicates that this registration is root.
    pub(crate) prv_txn_id: Option<Vec<u8>>,
    /// The purpose of the RBAC registration.
    pub(crate) purpose: Option<String>,
    /// A collection of error associate with this RBAC registration.
    /// If None, it can indicates that this registration is valid.
    /// Otherwise, invalid.
    pub(crate) problem_report: Option<String>,
}

impl RbacDbData {
    /// Create a new instance of the `RbacDbData`.
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
