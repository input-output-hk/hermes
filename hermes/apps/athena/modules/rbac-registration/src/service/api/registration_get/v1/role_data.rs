use std::collections::HashMap;

use anyhow::Context;
use cardano_blockchain_types::{Network, Slot};
use rbac_registration::{
    cardano::cip509::{PointData, RoleData},
    registration::cardano::RegistrationChain,
};
use serde::Serialize;

use crate::service::registration_get::v1::{
    extended_data::ExtendedData, key_data::KeyData, payment_data::PaymentData,
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RoleMap(HashMap<RoleId, RbacRoleData>);

impl From<HashMap<RoleId, RbacRoleData>> for RoleMap {
    fn from(value: HashMap<RoleId, RbacRoleData>) -> Self {
        Self(value)
    }
}

/// A RBAC registration role data.
#[derive(Debug, Clone, Serialize)]
pub struct RbacRoleData {
    /// A list of role signing keys.
    signing_keys: Vec<KeyData>,
    /// A list of role encryption keys.
    encryption_keys: Vec<KeyData>,
    /// A list of role payment addresses.
    payment_addresses: Vec<PaymentData>,
    /// A map of the extended data.
    ///
    /// Unlike other fields, we don't track history for this data.
    extended_data: ExtendedData,
}

impl RbacRoleData {
    /// Creates a new `RbacRoleData` instance.
    pub fn new(
        point_data: &[PointData<RoleData>],
        last_persistent_slot: Slot,
        chain: &RegistrationChain,
        network: Network,
    ) -> anyhow::Result<Self> {
        let mut signing_keys = Vec::new();
        let mut encryption_keys = Vec::new();
        let mut payment_addresses = Vec::new();
        let mut extended_data = HashMap::new();

        for point in point_data {
            let slot = point.point().slot_or_default();
            let is_persistent = slot <= last_persistent_slot;
            let time = network.slot_to_time(slot);
            let data = point.data();

            if let Some(signing_key) = data.signing_key() {
                signing_keys.push(
                    KeyData::new(is_persistent, time, signing_key, point.point(), chain)
                        .context("Invalid signing key")?,
                );
            }
            if let Some(encryption_key) = data.encryption_key() {
                encryption_keys.push(
                    KeyData::new(is_persistent, time, encryption_key, point.point(), chain)
                        .context("Invalid encryption key")?,
                );
            }
            payment_addresses.push(
                PaymentData::new(is_persistent, time, data.payment_key().cloned())
                    .context("Invalid payment address")?,
            );
            extended_data.extend(data.extended_data().clone().into_iter());
        }

        Ok(Self {
            signing_keys: signing_keys.into(),
            encryption_keys: encryption_keys.into(),
            payment_addresses: payment_addresses.into(),
            extended_data: extended_data.into(),
        })
    }
}
