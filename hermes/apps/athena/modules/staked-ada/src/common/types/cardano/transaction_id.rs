//! Transaction ID.

use cardano_blockchain_types::hashes::{TransactionId, BLAKE_2B256_SIZE};

use crate::{common::types::string_types::impl_string_types, utils::hex::as_hex_string};

/// Length of the hash itself;
const HASH_LENGTH: usize = BLAKE_2B256_SIZE;

impl_string_types!(TxnId, "string", "hex:hash(32)", is_valid);

impl TryFrom<Vec<u8>> for TxnId {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() != HASH_LENGTH {
            anyhow::bail!("Hash Length Invalid.")
        }
        Ok(Self(as_hex_string(&value)))
    }
}

impl From<TransactionId> for TxnId {
    fn from(value: TransactionId) -> Self {
        Self(value.to_string())
    }
}
