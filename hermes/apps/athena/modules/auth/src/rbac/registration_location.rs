//! The location of the registration.

/// Information needed to build the RBAC chain.
/// Only need the `slot_no` and `txn_idx` to construct a block and
/// extract the RBAC information.
#[derive(Debug, Clone)]
pub(crate) struct RegistrationLocation {
    /// The slot number of the block that contains the registration.
    pub(crate) slot_no: u64,
    /// The transaction index that contains the registration.
    pub(crate) txn_idx: u16,
}
