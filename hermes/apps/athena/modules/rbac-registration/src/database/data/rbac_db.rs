//! Data for the `rbac_registration` table.

/// Data of the `rbac_registration` table.
#[derive(Debug, Clone)]
pub(crate) struct RbacDbData {
    /// 32 bytes transaction ID (aka transaction hash).
    pub(crate) txn_id: Vec<u8>,
    /// Optional Catalyst short ID - this only exist for Role 0.
    pub(crate) catalyst_id: Option<String>,
    /// Slot number.
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
