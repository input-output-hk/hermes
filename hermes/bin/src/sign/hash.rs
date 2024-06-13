//! Blake2b-256 hash implementation.

/// Blake2b-256 hash instance.
/// Wrapper over `blake2b_simd::Hash`
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Hash(blake2b_simd::Hash);

impl Hash {
    /// Blake2b-256 hash size.
    const HASH_SIZE: usize = 32;

    /// Calculate a new `Hash` from bytes.
    pub(crate) fn calc_hash(bytes: &[u8]) -> Self {
        Self(
            blake2b_simd::Params::new()
                .hash_length(Self::HASH_SIZE)
                .hash(bytes),
        )
    }

    /// Convert the hash to a hexadecimal string.
    pub(crate) fn to_hex(&self) -> String {
        let bytes = self.0.as_bytes();
        hex::encode(bytes)
    }

    /// Convert the hash from a hexadecimal string.
    pub(crate) fn from_hex(s: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(s)?;
        let bytes: &[u8; Self::HASH_SIZE * 2] = bytes.as_slice().try_into()?;
        Ok(Self(bytes.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_serde_test() {
        let hash = Hash::calc_hash(b"test");

        let hex = hash.to_hex();
        assert_eq!(
            "928b20366943e2afd11ebc0eae2e53a93bf177a4fcf35bcc64d503704e65e202",
            hex
        );

        let decoded_hash = Hash::from_hex(&hex).expect("Could not decode hash from hex.");
        assert_eq!(hash, decoded_hash);
    }
}
