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
    block: Result<pallas::ledger::traverse::MultiEraBlock<'this>, pallas::ledger::traverse::Error>,
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
    pub fn new<'a>(raw_data: Vec<u8>) -> Result<Self, Error> {
        let builder = SelfReferencedMultiEraBlockBuilder {
            raw_data,
            block_builder: |raw_data| pallas::ledger::traverse::MultiEraBlock::decode(raw_data),
        };
        let self_ref_block = builder.build();
        if let Err(err) = &self_ref_block.borrow_block() {
            return Err(Error::Codec(err.to_string()));
        }
        Ok(Self(Arc::new(self_ref_block)))
    }

    /// Decodes the data into a multi-era block.
    ///
    /// # Panics
    ///
    /// If the data has changed between the creation of this `MultiEraBlockData` and now,
    /// it may panic.
    pub fn decode(&self) -> &pallas::ledger::traverse::MultiEraBlock {
        #[allow(clippy::unwrap_used)]
        // We checked the block before during construction, so it is safe to unwrap.
        self.0.borrow_block().as_ref().unwrap()
    }
}
