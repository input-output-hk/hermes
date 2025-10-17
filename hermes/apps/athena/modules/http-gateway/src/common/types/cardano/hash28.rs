//! Hex encoded 28 byte hash.
//!
//! Hex encoded string which represents a 28 byte hash.

use anyhow::bail;
use cardano_blockchain_types::hashes::BLAKE_2B224_SIZE;

use crate::{
    common::types::string_types::impl_string_types,
    utils::hex::{as_hex_string, from_hex_string},
};

/// Length of the hash itself;
const HASH_LENGTH: usize = BLAKE_2B224_SIZE;
impl_string_types!(HexEncodedHash28, "string", "hex:hash(28)", is_valid);

impl TryFrom<&Vec<u8>> for HexEncodedHash28 {
    type Error = anyhow::Error;

    fn try_from(value: &Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() != HASH_LENGTH {
            bail!("Hash Length Invalid.")
        }
        Ok(Self(as_hex_string(value)))
    }
}

// Because it is impossible for the Encoded Hash to not be valid (due to `is_valid`), we
// can ensure this method is infallible.
// All creation of this type should come only from one of the deserialization methods.
impl From<HexEncodedHash28> for Vec<u8> {
    fn from(val: HexEncodedHash28) -> Self {
        #[allow(clippy::expect_used)]
        from_hex_string(&val.0).expect("This can only fail if the type was invalidly constructed.")
    }
}
