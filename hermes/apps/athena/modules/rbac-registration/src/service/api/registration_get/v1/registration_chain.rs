//! RBAC registration V1 chain.

use serde::Serialize;

use crate::service::{
    common::{catalyst_id::CatalystId, transaction_id::TxnId, uuidv4::Uuidv4},
    registration_get::v1::{chain_info::ChainInfo, role_data::RoleMap},
};

/// A chain of valid RBAC registration.
#[derive(Debug, Clone, Serialize)]
pub struct RbacRegistrationChain {
    /// A Catalyst ID.
    pub(crate) catalyst_id: CatalystId,
    /// An ID of the last persistent transaction.
    pub(crate) last_persistent_txn: Option<TxnId>,
    /// An ID of the last volatile transaction.
    pub(crate) last_volatile_txn: Option<TxnId>,
    /// A list of registration purposes.
    pub(crate) purpose: Vec<Uuidv4>,
    /// A map of role number to role data.
    pub(crate) roles: RoleMap,
}

impl RbacRegistrationChain {
    /// Creates a new registration chain instance.
    pub fn new(info: &ChainInfo) -> anyhow::Result<Self> {
        let catalyst_id = info.chain.catalyst_id().clone().into();

        let last_persistent_txn: Option<TxnId> = info.last_persistent_txn.map(Into::into);
        let last_volatile_txn: Option<TxnId> = info.last_volatile_txn.map(Into::into);
        let purpose = info
            .chain
            .purpose()
            .iter()
            .copied()
            .map(Uuidv4::from)
            .collect::<Vec<_>>()
            .into();
        let roles = role_data(info)?.into();

        Ok(Self {
            catalyst_id,
            last_persistent_txn,
            last_volatile_txn,
            purpose,
            roles,
        })
    }
}

/// Gets and converts a role data from the given chain info.
fn role_data(info: &ChainInfo) -> anyhow::Result<HashMap<RoleId, RbacRoleData>> {
    info.chain
        .role_data_history()
        .iter()
        .map(|(&role, data)| {
            RbacRoleData::new(data, info.last_persistent_slot, &info.chain).map(|rbac| (role, rbac))
        })
        .collect()
}
