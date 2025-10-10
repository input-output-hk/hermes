//! Transaction ID.

use cardano_blockchain_types::hashes::{TransactionId, BLAKE_2B256_SIZE};
use serde::Serialize;

/// A Cardano transaction ID.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct TxnId(String);

impl TryFrom<Vec<u8>> for TxnId {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() != BLAKE_2B256_SIZE {
            anyhow::bail!("Hash Length Invalid.")
        }
        let v = format!("0x{}", hex::encode(value));
        Ok(Self(v))
    }
}

impl From<TransactionId> for TxnId {
    fn from(value: TransactionId) -> Self {
        Self(value.to_string())
    }
}
