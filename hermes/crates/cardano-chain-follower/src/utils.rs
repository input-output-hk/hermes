//! Simple general purpose utility functions.

use blake2b_simd::{self, Params};

/// Convert T to an i16. (saturate if out of range.)
#[allow(dead_code)] // Its OK if we don't use this general utility function.
pub(crate) fn i16_from_saturating<T: TryInto<i16>>(value: T) -> i16 {
    match value.try_into() {
        Ok(value) => value,
        Err(_) => i16::MAX,
    }
}

/// Convert an <T> to usize. (saturate if out of range.)
#[allow(dead_code)] // Its OK if we don't use this general utility function.
pub(crate) fn usize_from_saturating<
    T: Copy
        + TryInto<usize>
        + std::ops::Sub<Output = T>
        + std::cmp::PartialOrd<T>
        + num_traits::identities::Zero,
>(
    value: T,
) -> usize {
    if value < T::zero() {
        usize::MIN
    } else {
        match value.try_into() {
            Ok(value) => value,
            Err(_) => usize::MAX,
        }
    }
}

/// Convert an <T> to u32. (saturate if out of range.)
#[allow(dead_code)] // Its OK if we don't use this general utility function.
pub(crate) fn u32_from_saturating<
    T: Copy
        + TryInto<u32>
        + std::ops::Sub<Output = T>
        + std::cmp::PartialOrd<T>
        + num_traits::identities::Zero,
>(
    value: T,
) -> u32 {
    if value < T::zero() {
        u32::MIN
    } else {
        match value.try_into() {
            Ok(converted) => converted,
            Err(_) => u32::MAX,
        }
    }
}

/// Convert the given value to `blake2b_244` array.
#[allow(dead_code)] // Its OK if we don't use this general utility function.
pub(crate) fn blake2b_244(value: &[u8]) -> anyhow::Result<[u8; 28]> {
    let h = Params::new().hash_length(28).hash(value);
    let b = h.as_bytes();
    b.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid length of blake2b_244, expected 28 got {}", b.len()))
}

/// Convert the given value to `blake2b_256` array.
#[allow(dead_code)] // Its OK if we don't use this general utility function.
pub(crate) fn blake2b_256(value: &[u8]) -> anyhow::Result<[u8; 32]> {
    let h = Params::new().hash_length(32).hash(value);
    let b = h.as_bytes();
    b.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid length of blake2b_256, expected 32 got {}", b.len()))
}
