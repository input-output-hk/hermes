//! Simple general purpose utility functions.
use blake2b_simd::{self, Params};
use regex::Regex;

use crate::witness::TxWitness;

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

/// Extracts the CIP-19 bytes from a URI.
/// Example input: "web+cardano://addr/<cip-19 address string>"
/// https://github.com/cardano-foundation/CIPs/tree/6bae5165dde5d803778efa5e93bd408f3317ca03/CPS-0016
/// URI = scheme ":" ["//" authority] path ["?" query] ["#" fragment]
#[allow(dead_code)]
pub(crate) fn extract_cip19_hash(uri: &str) -> Option<Vec<u8>> {
    // Define a regex pattern to match the expected URI format
    let re = Regex::new(r"^.+://addr/(.+)$").ok()?;

    // Apply the regex pattern to capture the CIP-19 address string
    let address = re
        .captures(uri)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()));

    match address {
        Some(addr) => {
            let addr = bech32::decode(&addr).ok()?.1;
            // As in CIP19, the first byte is the header, so extract only the payload
            // TODO - This won't work with payment key
            Some(addr[1..].to_vec())
        },
        None => None,
    }
}

/// Compare the given public key bytes with the transaction witness set.
pub(crate) fn compare_key_hash(
    pk_addrs: Vec<Vec<u8>>, witness: TxWitness, txn_idx: u8,
) -> anyhow::Result<()> {
    pk_addrs.into_iter().try_for_each(|pk_addr| {
        let pk_addr: [u8; 28] = pk_addr.as_slice().try_into().map_err(|_| {
            anyhow::anyhow!(
                "Invalid length for vkey, expected 28 bytes but got {}",
                pk_addr.len()
            )
        })?;

        // Key hash not found in the transaction witness set
        if !witness.check_witness_in_tx(&pk_addr, txn_idx) {
            return Err(anyhow::anyhow!(
                "Public key hash not found in transaction witness set given {:?}",
                pk_addr
            ));
        }

        Ok(())
    })
}
