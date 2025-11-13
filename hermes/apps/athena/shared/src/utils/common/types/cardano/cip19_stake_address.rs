//! Cardano stake address types.
//!
//! More information can be found in [CIP-19](https://cips.cardano.org/cip/CIP-19)

use anyhow::bail;
use cardano_blockchain_types::{StakeAddress, pallas_addresses::Address};

use crate::utils::common::types::string_types::impl_string_types;

// cSpell:enable
/// Production Stake Address Identifier
const PROD_STAKE: &str = "stake";
/// Test Stake Address Identifier
const TEST_STAKE: &str = "stake_test";
/// Length of the decoded address.
const DECODED_ADDR_LEN: usize = 29;

impl_string_types!(Cip19StakeAddress, "string", FORMAT, is_valid);

impl TryFrom<&str> for Cip19StakeAddress {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.to_string().try_into()
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
                bail!("Invalid CIP-19 formatted Stake Address")
            },
            Err(err) => {
                bail!("Invalid CIP-19 formatted Stake Address : {err}");
            },
        };
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    // Test Vector: <https://cips.cardano.org/cip/CIP-19>
    // cspell: disable
    const VALID_PROD_STAKE_ADDRESS: &str =
        "stake1uyehkck0lajq8gr28t9uxnuvgcqrc6070x3k9r8048z8y5gh6ffgw";
    const VALID_TEST_STAKE_ADDRESS: &str =
        "stake_test1uqehkck0lajq8gr28t9uxnuvgcqrc6070x3k9r8048z8y5gssrtvn";
    const INVALID_STAKE_ADDRESS: &str =
        "invalid1u9nlq5nmuzthw3vhgakfpxyq4r0zl2c0p8uqy24gpyjsa6c3df4h6";
    // cspell: enable

    #[test]
    fn test_valid_stake_address_from_string() {
        let stake_address_prod = Cip19StakeAddress::try_from(VALID_PROD_STAKE_ADDRESS.to_string());
        let stake_address_test = Cip19StakeAddress::try_from(VALID_TEST_STAKE_ADDRESS.to_string());

        assert!(stake_address_prod.is_ok());
        assert!(stake_address_test.is_ok());
        assert_eq!(stake_address_prod.unwrap().0, VALID_PROD_STAKE_ADDRESS);
        assert_eq!(stake_address_test.unwrap().0, VALID_TEST_STAKE_ADDRESS);
    }

    #[test]
    fn test_invalid_stake_address_from_string() {
        let stake_address = Cip19StakeAddress::try_from(INVALID_STAKE_ADDRESS.to_string());
        assert!(stake_address.is_err());
    }

    #[test]
    fn cip19_stake_address_to_stake_address() {
        let stake_address_prod =
            Cip19StakeAddress::try_from(VALID_PROD_STAKE_ADDRESS.to_string()).unwrap();

        let stake_addr: StakeAddress = stake_address_prod.try_into().unwrap();
        let bytes = Vec::from(stake_addr);
        assert_eq!(bytes.len(), 29);
    }
}
