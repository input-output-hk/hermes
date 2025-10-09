//! RBAC chain metadata.

use cardano_blockchain_types::{hashes::TransactionId, Slot};

#[derive(Debug, Clone, Default)]
pub(crate) struct RbacChainMetadata {
    pub(crate) last_persistent_txn: Option<TransactionId>,
    pub(crate) last_volatile_txn: Option<TransactionId>,
    pub(crate) last_persistent_slot: Slot,
}
