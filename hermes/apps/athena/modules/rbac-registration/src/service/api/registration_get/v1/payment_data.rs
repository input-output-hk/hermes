//! A role payment address information.

use cardano_blockchain_types::pallas_addresses::ShelleyAddress;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::service::common::types::{
    cardano::cip19_shelley_address::Cip19ShelleyAddress,
    generic::date_time::DateTime as ServiceDateTime,
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
            is_persistent,
            time: time.into(),
            address,
        })
    }
}
