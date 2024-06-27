//! Multi Era CBOR Encoded Block Data

use std::sync::Arc;

use ouroboros::self_referencing;

use crate::error::Error;

/// Self-referencing CBOR encoded data of a multi-era block.
#[self_referencing]
#[derive(Debug)]
struct SelfReferencedMultiEraBlock {
    /// The CBOR encoded data of a multi-era block.
    raw_data: Vec<u8>,

    /// The decoded multi-era block.
    /// References the `raw_data` field.
    #[borrows(raw_data)]
    #[covariant]
    block: pallas::ledger::traverse::MultiEraBlock<'this>,
}

/// Multi-era block.
#[derive(Clone, Debug)]
pub struct MultiEraBlock(Arc<SelfReferencedMultiEraBlock>);

impl MultiEraBlock {
    /// Creates a new `MultiEraBlockData` from the given bytes.
    ///
    /// # Errors
    ///
    /// If the given bytes cannot be decoded as a multi-era block, an error is returned.
    pub fn new(raw_data: Vec<u8>) -> Result<Self, Error> {
        let builder = SelfReferencedMultiEraBlockTryBuilder {
            raw_data,
            block_builder: |raw_data| -> Result<_, Error> {
                pallas::ledger::traverse::MultiEraBlock::decode(raw_data).map_err(Error::Codec)
            },
        };
        let self_ref_block = builder.try_build()?;
        Ok(Self(Arc::new(self_ref_block)))
    }

    /// Decodes the data into a multi-era block.
    #[must_use]
    pub fn decode(&self) -> &pallas::ledger::traverse::MultiEraBlock {
        self.0.borrow_block()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_era_block_test() {
        let blocks_hex_bytes = vec![
            include_str!("./../test_data/byron.block"),
            include_str!("./../test_data/shelley.block"),
            include_str!("./../test_data/mary.block"),
            include_str!("./../test_data/allegra.block"),
            include_str!("./../test_data/alonzo.block"),
        ];

        for block_hex_bytes in blocks_hex_bytes {
            let block_bytes = hex::decode(block_hex_bytes).expect("Failed to decode hex block.");
            let block = MultiEraBlock::new(block_bytes.clone()).expect("Failed to decode block.");
            let pallas_block =
                pallas::ledger::traverse::MultiEraBlock::decode(block_bytes.as_slice())
                    .expect("Failed to decode pallas block.");

            assert_eq!(block.decode().hash(), pallas_block.hash());
        }
    }
}
