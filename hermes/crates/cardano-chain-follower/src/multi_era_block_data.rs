//! Multi Era CBOR Encoded Block Data

use crate::error::{Error, Result};
use pallas::ledger::traverse::MultiEraBlock;

/// CBOR encoded data of a multi-era block.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct MultiEraBlockData(Vec<u8>);

impl MultiEraBlockData {
    /// Creates a new `MultiEraBlockData` from the given bytes.
    #[must_use]
    pub fn new(data: Vec<u8>) -> Self {
        MultiEraBlockData(data)
    }

    /// Decodes the data into a multi-era block.
    ///
    /// # Errors
    ///
    /// Returns Err if the block's era couldn't be decided or if the encoded data is
    /// invalid.
    pub fn decode(&self) -> Result<MultiEraBlock> {
        let block = MultiEraBlock::decode(&self.0).map_err(Error::Codec)?;

        Ok(block)
    }

    /// Consumes the [`MultiEraBlockData`] returning the block data raw bytes.
    #[must_use]
    pub fn into_raw_data(self) -> Vec<u8> {
        self.0
    }

    /// Validate a multi-era block.
    ///
    /// This does not execute Plutus scripts nor validates ledger state.
    /// It only checks that the block is correctly formatted for its era.
    ///
    /// # Errors
    ///
    /// Returns Err if the block's era couldn't be decided or if the encoded data is invalid.
    pub fn validate(&self) -> Result<()> {
        self.decode()?;

        Ok(())
    }
}

impl AsRef<[u8]> for MultiEraBlockData {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
