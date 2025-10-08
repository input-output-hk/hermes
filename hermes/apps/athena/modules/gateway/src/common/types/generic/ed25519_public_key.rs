//! Ed25519 Public Key Type.
//!
//! Hex encoded string which represents an Ed25519 public key.

use std::sync::LazyLock;

use anyhow::bail;
use regex::Regex;

use crate::{
    common::types::string_types::impl_string_types, utilities::as_hex_string, utils::ed25519,
};

/// Validation Regex Pattern
pub(crate) const PATTERN: &str = "^0x[A-Fa-f0-9]{64}$";

/// Validate `Ed25519HexEncodedPublicKey` This part is done separately from the `PATTERN`
fn is_valid(hex_key: &str) -> bool {
    /// Regex to validate `Ed25519HexEncodedPublicKey`
    #[allow(clippy::unwrap_used)] // Safe because the Regex is constant.
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(PATTERN).unwrap());

    if RE.is_match(hex_key) {
        return ed25519::verifying_key_from_hex(hex_key).is_ok();
    }
    false
}

impl_string_types!(Ed25519HexEncodedPublicKey, "string", FORMAT, is_valid);

impl TryFrom<&str> for Ed25519HexEncodedPublicKey {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.to_string().try_into()
    }
}

impl TryFrom<String> for Ed25519HexEncodedPublicKey {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if !is_valid(&value) {
            bail!("Invalid Ed25519 Public key")
        }
        Ok(Self(value))
    }
}

impl TryFrom<Vec<u8>> for Ed25519HexEncodedPublicKey {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let key = ed25519::verifying_key_from_vec(&value)?;
        Ok(key.into())
    }
}

// Because it is impossible for the Encoded Key to not be a valid Verifying Key, we can
// ensure this method is infallible.
// All creation of this type should come from TryFrom<Vec<u8>>, or one of the
// deserialization methods.
impl From<Ed25519HexEncodedPublicKey> for ed25519_dalek::VerifyingKey {
    fn from(val: Ed25519HexEncodedPublicKey) -> Self {
        #[allow(clippy::expect_used)]
        ed25519::verifying_key_from_hex(&val.0)
            .expect("This can only fail if the type was invalidly constructed.")
    }
}

impl From<ed25519_dalek::VerifyingKey> for Ed25519HexEncodedPublicKey {
    fn from(key: ed25519_dalek::VerifyingKey) -> Self {
        Self(as_hex_string(key.as_ref()))
    }
}

impl TryInto<Vec<u8>> for Ed25519HexEncodedPublicKey {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        Ok(hex::decode(self.0.trim_start_matches("0x"))?)
    }
}
