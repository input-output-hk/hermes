//! Ed25519 Public Key Type.
//!
//! Hex encoded string which represents an Ed25519 public key.

use crate::service::utilities::as_hex_string;

/// Ed25519 Public Key.
#[derive(Clone, Debug)]
pub(crate) struct Ed25519HexEncodedPublicKey(String);

impl From<ed25519_dalek::VerifyingKey> for Ed25519HexEncodedPublicKey {
    fn from(key: ed25519_dalek::VerifyingKey) -> Self {
        Self(as_hex_string(&key.to_bytes()))
    }
}

impl From<Ed25519HexEncodedPublicKey> for String {
    fn from(val: Ed25519HexEncodedPublicKey) -> Self {
        val.0
    }
}
