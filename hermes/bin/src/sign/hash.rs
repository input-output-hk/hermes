//! Blake2b-256 hash implementation.

/// Blake2b-256 hash size.
const HASH_SIZE: usize = 32;

/// Blake2b-256 hasher instance.
pub(crate) struct Blake2b256Hasher(blake2b_simd::State);

impl Blake2b256Hasher {
    /// Create a new `Blake2b256Hasher`.
    pub(crate) fn new() -> Self {
        Self(
            blake2b_simd::Params::new()
                .hash_length(HASH_SIZE)
                .to_state(),
        )
    }

    /// Incrementally add bytes to the hasher.
    pub(crate) fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    ///  Finalize the state and return a `Hash`.
    pub(crate) fn finalize(self) -> Blake2b256 {
        let hash = self.0.finalize();
        Blake2b256::from_bytes(hash.as_bytes())
    }
}

/// Blake2b-256 hash instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Blake2b256([u8; HASH_SIZE]);

impl Blake2b256 {
    /// Create a new `Blake2b256` from bytes.
    /// It's not doing any validation of the bytes size, so all checks should be done by
    /// the caller.
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut hash_bytes = [0; HASH_SIZE];
        hash_bytes.copy_from_slice(bytes);

        Self(hash_bytes)
    }

    /// Calculate a new `Blake2b256` from bytes.
    pub(crate) fn hash(bytes: &[u8]) -> Self {
        let hash = blake2b_simd::Params::new()
            .hash_length(HASH_SIZE)
            .hash(bytes);

        Self::from_bytes(hash.as_bytes())
    }

    /// Convert the hash to a hexadecimal string.
    pub(crate) fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Return the hash bytes.
    pub(crate) fn to_bytes(&self) -> [u8; HASH_SIZE] {
        self.0
    }

    /// Convert the hash from a hexadecimal string.
    pub(crate) fn from_hex(s: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(s)?;
        anyhow::ensure!(
            bytes.len() == HASH_SIZE,
            "Invalid hash length: expected {}, provided {}.",
            HASH_SIZE,
            bytes.len()
        );
        Ok(Self::from_bytes(&bytes))
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

    #[test]
    fn hasher_test() {
        let hasher = Blake2b256Hasher::new();
        // hasher.update(b"test");
        let hash = hasher.finalize();
        assert_eq!(
            "928b20366943e2afd11ebc0eae2e53a93bf177a4fcf35bcc64d503704e65e202",
            hash.to_hex()
        );
    }
}
