//! RBAC registration V1 chain.

use std::collections::HashMap;

use catalyst_types::catalyst_id::role_index::RoleId;
use serde::Serialize;

use crate::service::{
    api::registration_get::v1::{
        chain_info::ChainInfo, role_data::RbacRoleData, role_map::RoleMap,
    },
    common::types::{
        cardano::{catalyst_id::CatalystId, transaction_id::TxnId},
        generic::uuidv4::UUIDv4,
    },
};

/// A chain of valid RBAC registration.
///
/// A unified data of multiple RBAC registrations.
#[derive(Debug, Clone, Serialize)]
pub struct RbacRegistrationChain {
    /// A Catalyst ID.
    pub(crate) catalyst_id: CatalystId,
    /// An ID of the last persistent transaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) last_persistent_txn: Option<TxnId>,
    /// An ID of the last volatile transaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) last_volatile_txn: Option<TxnId>,
    /// A list of registration purposes.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) purpose: Vec<UUIDv4>,
    /// A map of role number to role data.
    // This map should not be empty
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
            .map(|uuid| {
                let uuid_str = uuid.to_string();
                UUIDv4::try_from(uuid_str.as_str()).map_err(anyhow::Error::msg)
            })
            .collect::<Result<Vec<_>, _>>()?
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
            RbacRoleData::new(data, info.last_persistent_slot, &info.chain, info.network)
                .map(|rbac| (role, rbac))
        })
        .collect()
}
