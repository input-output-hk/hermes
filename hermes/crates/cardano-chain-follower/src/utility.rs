//! Utility functions

use blake2b_simd::{self, Params};

/// Convert the given value to `blake2b_244` array.
pub fn blake2b_244(value: &[u8]) -> anyhow::Result<[u8; 28]> {
    let h = Params::new().hash_length(28).hash(value);
    let b = h.as_bytes();
    b.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid length of blake2b_244, expected 28 got {}", b.len()))
}

#[allow(dead_code)]
/// Convert the given value to `blake2b_256` array.
pub fn blake2b_256(value: &[u8]) -> anyhow::Result<[u8; 32]> {
    let h = Params::new().hash_length(32).hash(value);
    let b = h.as_bytes();
    b.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid length of blake2b_256, expected 32 got {}", b.len()))
}
