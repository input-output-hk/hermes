//! Simple general purpose utility functions.

use blake2b_simd::{self, Params};

/// Convert T to an i16. (saturate if out of range.)
#[allow(dead_code)]
pub(crate) fn i16_from_saturating<T: TryInto<i16>>(value: T) -> i16 {
    match value.try_into() {
        Ok(value) => value,
        Err(_) => i16::MAX,
    }
}

/// Convert an i16 to usize. (saturate if out of range.)
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

/// Convert the given value to `blake2b_244` array.
pub(crate) fn blake2b_244(value: &[u8]) -> anyhow::Result<[u8; 28]> {
    let h = Params::new().hash_length(28).hash(value);
    let b = h.as_bytes();
    b.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid length of blake2b_244, expected 28 got {}", b.len()))
}

#[allow(dead_code)]
/// Convert the given value to `blake2b_256` array.
pub(crate) fn blake2b_256(value: &[u8]) -> anyhow::Result<[u8; 32]> {
    let h = Params::new().hash_length(32).hash(value);
    let b = h.as_bytes();
    b.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid length of blake2b_256, expected 32 got {}", b.len()))
}

#[allow(dead_code)]
/// Convert the given value to `blake2b_128` array.
pub(crate) fn blake2b_128(value: &[u8]) -> anyhow::Result<[u8; 16]> {
    let h = Params::new().hash_length(16).hash(value);
    let b = h.as_bytes();
    b.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid length of blake2b_128, expected 16 got {}", b.len()))
}

/// Cardano chain.
const CARDANO: &str = "cardano";

/// Extracting public key from dns.
/// eg. cardano://....
pub(crate) fn extract_pk_dns(domain: &str) -> Option<String> {
    let mut p = domain.split("://");
    let chain = p.next()?;
    let pk = p.next()?;
    if chain == CARDANO {
        Some(pk.to_string())
    } else {
        None
    }
}
