//! `API` Utility operations

/// Convert bytes to hex string with the `0x` prefix
pub(crate) fn as_hex_string(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}
