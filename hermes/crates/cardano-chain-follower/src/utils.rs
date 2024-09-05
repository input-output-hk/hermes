//! Simple general purpose utility functions.
use blake2b_simd::{self, Params};
use regex::Regex;

use crate::witness::TxWitness;

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

/// Convert an <T> to u64. (saturate if out of range.)
#[allow(dead_code)] // Its OK if we don't use this general utility function.
pub(crate) fn u64_from_saturating<
    T: Copy
        + TryInto<u64>
        + std::ops::Sub<Output = T>
        + std::cmp::PartialOrd<T>
        + num_traits::identities::Zero,
>(
    value: T,
) -> u64 {
    if value < T::zero() {
        u64::MIN
    } else {
        match value.try_into() {
            Ok(converted) => converted,
            Err(_) => u64::MAX,
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

/// Convert the given value to `blake2b_256` array.
pub(crate) fn blake2b_256(value: &[u8]) -> anyhow::Result<[u8; 32]> {
    let h = Params::new().hash_length(32).hash(value);
    let b = h.as_bytes();
    b.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid length of blake2b_256, expected 32 got {}", b.len()))
}

/// Convert the given value to `blake2b_128` array.
pub(crate) fn blake2b_128(value: &[u8]) -> anyhow::Result<[u8; 16]> {
    let h = Params::new().hash_length(16).hash(value);
    let b = h.as_bytes();
    b.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid length of blake2b_128, expected 16 got {}", b.len()))
}

/// Extracts the CIP-19 bytes from a URI.
#[allow(clippy::doc_markdown)]
/// Example input: "web+cardano://addr/<cip-19 address string>"
/// <https://github.com/cardano-foundation/CIPs/tree/6bae5165dde5d803778efa5e93bd408f3317ca03/CPS-0016>
/// URI = scheme ":" ["//" authority] path ["?" query] ["#" fragment]
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
            extract_key_hash(&addr)
        },
        None => None,
    }
}

/// Extract the first 28 bytes from the given key
/// Refer to <https://cips.cardano.org/cip/CIP-19> for more information.
pub(crate) fn extract_key_hash(key: &[u8]) -> Option<Vec<u8>> {
    key.get(1..29).map(<[u8]>::to_vec)
}

/// Compare the given public key bytes with the transaction witness set.
pub(crate) fn compare_key_hash(
    pk_addrs: &[Vec<u8>], witness: &TxWitness, txn_idx: u8,
) -> anyhow::Result<()> {
    if pk_addrs.is_empty() {
        return Err(anyhow::anyhow!("No public key addresses provided"));
    }

    pk_addrs.iter().try_for_each(|pk_addr| {
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

/// Zero out the last n bytes
pub(crate) fn zero_out_last_n_bytes(vec: &mut [u8], n: usize) {
    if let Some(slice) = vec.get_mut(vec.len().saturating_sub(n)..) {
        slice.fill(0);
    }
}

/// Decode the given UTF-8 content.
pub(crate) fn decode_utf8(content: &[u8]) -> anyhow::Result<String> {
    // Decode the UTF-8 string
    std::str::from_utf8(content)
        .map(std::string::ToString::to_string)
        .map_err(|_| {
            anyhow::anyhow!(
                "Invalid UTF-8 string, expected valid UTF-8 string but got {:?}",
                content
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    // cspell: words stake_test1uqehkck0lajq8gr28t9uxnuvgcqrc6070x3k9r8048z8y5gssrtvn

    #[test]
    fn test_extract_cip19_hash() {
        // Test stake data from https://cips.cardano.org/cip/CIP-19
        // Additional tools to check for bech32 https://slowli.github.io/bech32-buffer/
        let uri =
            "web+cardano://addr/stake_test1uqehkck0lajq8gr28t9uxnuvgcqrc6070x3k9r8048z8y5gssrtvn";
        // Given:
        // e0337b62cfff6403a06a3acbc34f8c46003c69fe79a3628cefa9c47251
        // The first byte is the header, so extract only the payload
        let bytes = hex::decode("337b62cfff6403a06a3acbc34f8c46003c69fe79a3628cefa9c47251")
            .expect("Failed to decode bytes");
        assert_eq!(
            extract_cip19_hash(uri).expect("Failed to extract CIP-19 hash"),
            bytes
        );
    }

    #[test]
    fn test_extract_cip19_hash_invalid_uri() {
        let uri = "invalid_uri";
        let result = extract_cip19_hash(uri);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_cip19_hash_non_bech32_address() {
        let uri = "example://addr/not_bech32";
        let result = extract_cip19_hash(uri);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_cip19_hash_empty_uri() {
        let uri = "";
        let result = extract_cip19_hash(uri);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_cip19_hash_no_address() {
        let uri = "example://addr/";
        let result = extract_cip19_hash(uri);
        assert_eq!(result, None);
    }
}
