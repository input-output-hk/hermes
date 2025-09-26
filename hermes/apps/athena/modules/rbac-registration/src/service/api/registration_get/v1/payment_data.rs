//! A role payment address information.

use cardano_blockchain_types::pallas_addresses::ShelleyAddress;
use serde::Serialize;

use crate::service::{
    common::cip19_shelley_address::Cip19ShelleyAddress,
    registration_get::v1::types::ServiceDateTime,
};

/// A role payment address information.
#[derive(Debug, Clone, Serialize)]

pub(crate) struct PaymentData {
    /// Indicates if the data is persistent or volatile.
    is_persistent: bool,
    /// A time when the address was added.
    time: ServiceDateTime,
    /// An option payment address.
    address: Option<Cip19ShelleyAddress>,
}

impl PaymentData {
    /// Creates a new `PaymentData` instance.
    pub fn new(
        is_persistent: bool,
        time: DateTime<Utc>,
        address: Option<ShelleyAddress>,
    ) -> anyhow::Result<Self> {
        let address = address.map(Cip19ShelleyAddress::try_from).transpose()?;

        Ok(Self {
            is_persistent: is_persistent.into(),
            time: time.into(),
            address,
        })
    }
}
