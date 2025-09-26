//! A hex encoded binary data.

use serde::Serialize;

use crate::service::{
    common::types::generic::ed25519_public_keys::Ed25519HexEncodedPublicKey,
    utilities::as_hex_string,
};

#[derive(Clone, Debug, Serialize)]
pub(crate) struct HexEncodedBinaryData(String);

impl From<Vec<u8>> for HexEncodedBinaryData {
    fn from(value: Vec<u8>) -> Self {
        Self(as_hex_string(&value))
    }
}

impl From<Ed25519HexEncodedPublicKey> for HexEncodedBinaryData {
    fn from(value: Ed25519HexEncodedPublicKey) -> Self {
        Self(value.into())
    }
}
