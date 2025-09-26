//! Ed25519 Public Key Type.
//!
//! Hex encoded string which represents an Ed25519 public key.

#[derive(Clone, Debug)]
pub(crate) struct Ed25519HexEncodedPublicKey(String);

impl From<ed25519_dalek::VerifyingKey> for Ed25519HexEncodedPublicKey {
    fn from(key: ed25519_dalek::VerifyingKey) -> Self {
        let v = format!("0x{}", hex::encode(key));

        Self(v)
    }
}

impl From<String> for Ed25519HexEncodedPublicKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}
