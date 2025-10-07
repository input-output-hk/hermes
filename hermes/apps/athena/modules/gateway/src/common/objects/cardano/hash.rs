//! Defines API schema of Cardano hash type.

use core::fmt;
use std::str::FromStr;

use crate::utilities::as_hex_string;

/// Cardano Blake2b256 hash encoded in hex.
#[derive(Debug, Clone)]
pub(crate) struct Hash256([u8; Hash256::BYTE_LEN]);

impl Hash256 {
    /// The byte size for this hash.
    const BYTE_LEN: usize = 32;
    /// The hex-encoded hash length of this hash type.
    const HASH_LEN: usize = Self::BYTE_LEN * 2;
}

impl TryFrom<Vec<u8>> for Hash256 {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        value
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid {}-bit Cardano hash length.", Self::BYTE_LEN * 8))
            .map(Self)
    }
}

impl FromStr for Hash256 {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hash = s.strip_prefix("0x").ok_or(anyhow::anyhow!(
            "Invalid Cardano hash. Hex string must start with `0x`.",
        ))?;

        hex::decode(hash)
            .map_err(|_| anyhow::anyhow!("Invalid Cardano hash. Must be hex string."))?
            .try_into()
    }
}

impl AsRef<[u8]> for Hash256 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for Hash256 {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        as_hex_string(&self.0).fmt(f)
    }
}

/// Cardano Blake2b128 hash encoded in hex.
#[derive(Debug, Clone)]
pub(crate) struct Hash128([u8; Hash128::BYTE_LEN]);

impl Hash128 {
    /// The byte size for this hash.
    const BYTE_LEN: usize = 16;
    /// The hex-encoded hash length of this hash type.
    const HASH_LEN: usize = Self::BYTE_LEN * 2;
}

impl TryFrom<Vec<u8>> for Hash128 {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        value
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid {}-bit Cardano hash length.", Self::BYTE_LEN * 8))
            .map(Self)
    }
}

impl FromStr for Hash128 {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hash = s.strip_prefix("0x").ok_or(anyhow::anyhow!(
            "Invalid Cardano hash. Hex string must start with `0x`.",
        ))?;

        hex::decode(hash)
            .map_err(|_| anyhow::anyhow!("Invalid Cardano hash. Must be hex string."))?
            .try_into()
    }
}

impl AsRef<[u8]> for Hash128 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for Hash128 {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        as_hex_string(&self.0).fmt(f)
    }
}
