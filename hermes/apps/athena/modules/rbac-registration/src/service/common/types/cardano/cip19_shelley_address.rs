//! Cardano address types.
//!
//! More information can be found in [CIP-19](https://cips.cardano.org/cip/CIP-19)

use cardano_blockchain_types::pallas_addresses::{Address, ShelleyAddress};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Cip19ShelleyAddress(String);
impl TryFrom<Vec<u8>> for Cip19ShelleyAddress {
    type Error = anyhow::Error;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let addr = Address::from_bytes(&bytes)?;
        let Address::Shelley(addr) = addr else {
            return Err(anyhow::anyhow!("Not a Shelley address: {addr}"));
        };
        addr.try_into()
    }
}

impl TryFrom<ShelleyAddress> for Cip19ShelleyAddress {
    type Error = anyhow::Error;

    fn try_from(addr: ShelleyAddress) -> Result<Self, Self::Error> {
        let addr_str = addr
            .to_bech32()
            .map_err(|e| anyhow::anyhow!(format!("Invalid Shelley address {e}")))?;
        Ok(Self(addr_str))
    }
}

impl TryInto<ShelleyAddress> for Cip19ShelleyAddress {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<ShelleyAddress, Self::Error> {
        let address_str = &self.0;
        let address = Address::from_bech32(address_str)?;
        match address {
            Address::Shelley(address) => Ok(address),
            _ => Err(anyhow::anyhow!("Invalid Shelley address")),
        }
    }
}
