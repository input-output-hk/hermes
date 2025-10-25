//! RBAC chain metadata.

use cardano_blockchain_types::{hashes::TransactionId, Slot};

/// RBAC chain metadata.
#[derive(Debug, Clone, Default)]
pub(crate) struct RbacChainMetadata {
    /// Last persistent transaction.
    pub(crate) last_persistent_txn: Option<TransactionId>,
    /// Last volatile transaction.
    pub(crate) last_volatile_txn: Option<TransactionId>,
    /// Last persistent slot.
    pub(crate) last_persistent_slot: Slot,
}
