//! An update of a blockchain

use std::fmt::Display;

use pallas::network::miniprotocols::Point;
use strum::Display;

use crate::multi_era_block_data::MultiEraBlock;

/// Enum of chain updates received by the follower.
#[derive(Debug, Clone, Display, PartialEq)]
pub enum Type {
    /// Immutable Block from the immutable part of the blockchain.
    ImmutableBlock,
    /// A new part of the chain has become immutable (Roll-forward).
    ImmutableBlockRollForward,
    /// New block inserted on chain.
    Block,
    /// Chain rollback to the given block.
    Rollback,
}

/// Actual Chain Update itself.
#[derive(Clone)]
pub struct ChainUpdate {
    /// What point is this chain update for?
    pub point: Point,
    /// What kind of update is this?
    pub update: Type,
    /// Is this the tip of the chain?
    pub tip: bool,
    /// What is the new data?
    pub data: MultiEraBlock,
}

impl ChainUpdate {
    /// Creates a new chain update.
    #[must_use]
    pub fn new(update: Type, point: Point, tip: bool, data: MultiEraBlock) -> Self {
        Self {
            point,
            update,
            tip,
            data,
        }
    }

    /// Gets the chain update's block data.
    #[must_use]
    pub fn block_data(&self) -> &MultiEraBlock {
        &self.data
    }

    /// Gets the chain update's block data.
    #[must_use]
    pub fn immutable(&self) -> bool {
        match self.update {
            Type::ImmutableBlock | Type::ImmutableBlockRollForward => true,
            Type::Block | Type::Rollback => false,
        }
    }
}

impl Display for ChainUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let block_type = self.update.to_string();

        let block = self.block_data().decode();
        let block_number = block.number();
        let slot = block.slot();
        let size = block.size();
        let txns = block.tx_count();
        let aux_data = block.has_aux_data();

        let block_era = match block {
            pallas::ledger::traverse::MultiEraBlock::EpochBoundary(_) => {
                "Byron Epoch Boundary".to_string()
            },
            pallas::ledger::traverse::MultiEraBlock::AlonzoCompatible(_, era) => {
                format!("{era}")
            },
            pallas::ledger::traverse::MultiEraBlock::Babbage(_) => "Babbage".to_string(),
            pallas::ledger::traverse::MultiEraBlock::Byron(_) => "Byron".to_string(),
            pallas::ledger::traverse::MultiEraBlock::Conway(_) => "Conway".to_string(),
            _ => "Unknown".to_string(),
        };

        write!(f, "{block_type} {block_era} block : Slot# {slot} : Block# {block_number} : Size {size} : Txns {txns} : AuxData? {aux_data}")?;

        Ok(())
    }
}
