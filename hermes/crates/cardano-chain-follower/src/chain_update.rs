//! An update of a blockchain

use std::fmt::Display;

use strum::Display;

use crate::multi_era_block_data::MultiEraBlock;

/// Enum of chain updates received by the follower.
#[derive(Debug, Clone, Display, PartialEq)]
pub enum Kind {
    /// A new part of the chain has become immutable (Roll-forward).
    ImmutableBlockRollForward,
    /// New block inserted on chain.
    Block,
    /// Chain rollback to the given block.
    Rollback,
}

/// Actual Chain Update itself.
#[derive(Clone, Debug)]
pub struct ChainUpdate {
    /// What kind of update is this?
    pub kind: Kind,
    /// Is this the tip of the chain?
    pub tip: bool,
    /// What is the new data?
    pub data: MultiEraBlock,
}

impl ChainUpdate {
    /// Creates a new chain update.
    #[must_use]
    pub fn new(kind: Kind, tip: bool, data: MultiEraBlock) -> Self {
        Self { kind, tip, data }
    }

    /// Gets the chain update's block data.
    #[must_use]
    pub fn block_data(&self) -> &MultiEraBlock {
        &self.data
    }

    /// Gets the chain update's block data.
    #[must_use]
    pub fn immutable(&self) -> bool {
        self.data.immutable()
    }
}

impl Display for ChainUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let block_type = self.kind.to_string();
        let mut tip: String = String::new();
        if self.tip {
            tip = " @ Tip".to_string();
        }

        write!(f, "{block_type}{tip} : {}", self.data)?;

        Ok(())
    }
}
