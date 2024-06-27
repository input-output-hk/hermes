//! Multi Era CBOR Encoded Block Data

use pallas::ledger::traverse::MultiEraBlock;

use crate::error::{Error, Result};

/// CBOR encoded data of a multi-era block.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct MultiEraBlockData {
    /// The CBOR encoded data of a multi-era block.
    data: Vec<u8>,
}

impl MultiEraBlockData {
    /// Creates a new `MultiEraBlockData` from the given bytes.
    ///
    /// # Errors
    ///
    /// If the given bytes cannot be decoded as a multi-era block, an error is returned.
    pub fn new(data: Vec<u8>) -> Result<Self> {
        let tmp_block = MultiEraBlock::decode(&data).map_err(Error::Codec)?;
        drop(tmp_block);
        Ok(MultiEraBlockData { data })
    }

    /// Decodes the data into a multi-era block.
    ///
    /// # Panics
    ///
    /// If the data has changed between the creation of this `MultiEraBlockData` and now,
    /// it may panic.
    pub fn decode(&self) -> MultiEraBlock {
        #[allow(clippy::unwrap_used)]
        let block = MultiEraBlock::decode(&self.data)
            .map_err(Error::Codec)
            .unwrap();

        block
    }

    /// Consumes the [`MultiEraBlockData`] returning the block data raw bytes.
    #[must_use]
    pub fn into_raw_data(self) -> Vec<u8> {
        self.data
    }
}

impl AsRef<[u8]> for MultiEraBlockData {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}
