//! Blake2b-256 hash implementation.

/// Blake2b-256 hash instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Blake2b256([u8; Self::HASH_SIZE]);

impl Blake2b256 {
    /// Blake2b-256 hash size.
    const HASH_SIZE: usize = 32;

    /// Calculate a new `Blake2b256` from bytes.
    pub(crate) fn hash(bytes: &[u8]) -> Self {
        let hash = blake2b_simd::Params::new()
            .hash_length(Self::HASH_SIZE)
            .hash(bytes);

        let mut hash_bytes = [0; Self::HASH_SIZE];
        hash_bytes.copy_from_slice(hash.as_bytes());

        Self(hash_bytes)
    }

    /// Convert the hash to a hexadecimal string.
    pub(crate) fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Convert the hash from a hexadecimal string.
    pub(crate) fn from_hex(s: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(s)?;
        let hash_bytes = bytes.as_slice().try_into()?;
        Ok(Self(hash_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_serde_test() {
        let hash = Blake2b256::hash(b"test");

        let hex = hash.to_hex();
        assert_eq!(
            "928b20366943e2afd11ebc0eae2e53a93bf177a4fcf35bcc64d503704e65e202",
            hex
        );

        let decoded_hash = Blake2b256::from_hex(&hex).expect("Could not decode hash from hex.");
        assert_eq!(hash, decoded_hash);
    }
}
