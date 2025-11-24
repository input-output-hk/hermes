//! Hex helper functions

use anyhow::{Result, bail};

/// Convert bytes to hex string with the `0x` prefix
pub(crate) fn as_hex_string<T: AsRef<[u8]>>(bytes: T) -> String {
    format!("0x{}", hex::encode(bytes))
}

/// Convert bytes to hex string with the `0x` prefix
pub(crate) fn from_hex_string(hex: &str) -> Result<Vec<u8>> {
    #[allow(clippy::string_slice)] // Safe because of size checks.
    if hex.len() < 4 || hex.len() % 2 != 0 || &hex[0..2] != "0x" {
        bail!("Invalid hex string");
    }

    #[allow(clippy::string_slice)] // Safe due to above checks.
    Ok(hex::decode(&hex[2..])?)
}
