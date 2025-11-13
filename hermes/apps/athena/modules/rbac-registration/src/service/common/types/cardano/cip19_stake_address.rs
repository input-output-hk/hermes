//! Cardano stake address types.
//!
//! More information can be found in [CIP-19](https://cips.cardano.org/cip/CIP-19)

use cardano_blockchain_types::{StakeAddress, pallas_addresses::Address};

/// Length of the decoded address.
const DECODED_ADDR_LEN: usize = 29;

/// Production Stake Address Identifier
const PROD_STAKE: &str = "stake";

/// Test Stake Address Identifier
const TEST_STAKE: &str = "stake_test";

/// A Cardano stake address.
#[derive(Debug, Clone)]
pub(crate) struct Cip19StakeAddress(String);

impl From<StakeAddress> for Cip19StakeAddress {
    fn from(value: StakeAddress) -> Self {
        Self(value.to_string())
    }
}

impl TryInto<StakeAddress> for Cip19StakeAddress {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<StakeAddress, Self::Error> {
        let address_str = &self.0;
        let address = Address::from_bech32(address_str)?;
        match address {
            Address::Stake(address) => Ok(address.into()),
            _ => Err(anyhow::anyhow!("Invalid stake address")),
        }
    }
}

impl TryFrom<String> for Cip19StakeAddress {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match bech32::decode(&value) {
            Ok((hrp, addr)) => {
                let hrp = hrp.as_str();
                if addr.len() == DECODED_ADDR_LEN && (hrp == PROD_STAKE || hrp == TEST_STAKE) {
                    return Ok(Cip19StakeAddress(value));
                }
                anyhow::bail!("Invalid CIP-19 formatted Stake Address")
            },
            Err(err) => {
                anyhow::bail!("Invalid CIP-19 formatted Stake Address : {err}");
            },
        };
    }
}

impl TryFrom<&str> for Cip19StakeAddress {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.to_string().try_into()
    }
}
