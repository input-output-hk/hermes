//! A RBAC registration chain information.

use cardano_blockchain_types::{Network, Slot, hashes::TransactionId};
use rbac_registration::registration::cardano::RegistrationChain;

/// A RBAC registration chain along with additional information.
pub(crate) struct ChainInfo {
    /// A RBAC registration chain.
    pub(crate) chain: RegistrationChain,
    /// The latest persistent transaction ID of the chain.
    pub(crate) last_persistent_txn: Option<TransactionId>,
    /// The latest volatile transaction ID of the chain.
    pub(crate) last_volatile_txn: Option<TransactionId>,
    /// A slot number of the latest persistent registration.
    pub(crate) last_persistent_slot: Slot,
    /// The network of the chain.
    pub(crate) network: Network,
}
