//! Cardano address types.
//!
//! More information can be found in [CIP-19](https://cips.cardano.org/cip/CIP-19)

use std::sync::LazyLock;

use cardano_blockchain_types::{
    hashes::BLAKE_2B224_SIZE,
    pallas_addresses::{Address, ShelleyAddress},
};
use const_format::concatcp;
use regex::Regex;

use crate::common::types::string_types::impl_string_types;

/// Title
const TITLE: &str = "Cardano Payment Address";
/// Description
const DESCRIPTION: &str = "Cardano Shelley Payment Address (CIP-19 Formatted).";
/// Example
// cSpell:disable
const EXAMPLE: &str = "addr_test1qz2fxv2umyhttkxyxp8x0dlpdt3k6cwng5pxj3jhsydzer3n0d3vllmyqwsx5wktcd8cc3sq835lu7drv2xwl2wywfgs68faae";
// cSpell:enable
/// Production Address Identifier
const PROD: &str = "addr";
/// Test Address Identifier
const TEST: &str = "addr_test";
/// Bech32 Match Pattern
const BECH32: &str = "[a,c-h,j-n,p-z,0,2-9]";
/// Length of the encoded address (for type 0 - 3).
const ENCODED_STAKED_ADDR_LEN: usize = 98;
/// Length of the encoded address (for type 6 - 7).
const ENCODED_UNSTAKED_ADDR_LEN: usize = 53;
/// Regex Pattern
const PATTERN: &str = concatcp!(
    "^(",
    PROD,
    "|",
    TEST,
    ")1(",
    BECH32,
    "{",
    ENCODED_UNSTAKED_ADDR_LEN,
    "}|",
    BECH32,
    "{",
    ENCODED_STAKED_ADDR_LEN,
    "})$"
);

/// Header length
const HEADER_LEN: usize = 1;
/// Length of the decoded address.
const DECODED_UNSTAKED_ADDR_LEN: usize = BLAKE_2B224_SIZE;
/// Length of the decoded address.
const DECODED_STAKED_ADDR_LEN: usize = DECODED_UNSTAKED_ADDR_LEN * 2;
/// Minimum length
const MIN_LENGTH: usize = PROD.len() + 1 + ENCODED_UNSTAKED_ADDR_LEN;
/// Minimum length
const MAX_LENGTH: usize = TEST.len() + 1 + ENCODED_STAKED_ADDR_LEN;

/// Validate `Cip19ShelleyAddress` This part is done separately from the `PATTERN`
fn is_valid(addr: &str) -> bool {
    /// Regex to validate `Cip19ShelleyAddress`
    #[allow(clippy::unwrap_used)] // Safe because the Regex is constant.
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(PATTERN).unwrap());

    if RE.is_match(addr) {
        if let Ok((hrp, addr)) = bech32::decode(addr) {
            let hrp = hrp.as_str();
            return (addr.len() == (DECODED_UNSTAKED_ADDR_LEN + HEADER_LEN)
                || addr.len() == (DECODED_STAKED_ADDR_LEN + HEADER_LEN))
                && (hrp == PROD || hrp == TEST);
        }
    }
    false
}

impl_string_types!(
    Cip19ShelleyAddress,
    "string",
    "cardano:cip19-address",
    is_valid
);

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
