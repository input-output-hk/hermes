//! An update of a blockchain

use std::fmt::Display;

use crate::multi_era_block_data::MultiEraBlockData;

/// Enum of chain updates received by the follower.
#[derive(Debug)]
pub enum ChainUpdate {
    /// Immutable Block from the immutable part of the blockchain.
    ImmutableBlock(MultiEraBlockData),
    /// Immutable Block from the immutable part of the blockchain (Rollback).
    ImmutableBlockRollback(MultiEraBlockData),
    /// New block inserted on chain.
    Block(MultiEraBlockData),
    /// New block inserted on chain.
    BlockTip(MultiEraBlockData),
    /// Chain rollback to the given block.
    Rollback(MultiEraBlockData),
}

impl Display for ChainUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let block_type = match self {
            Self::ImmutableBlock(_) => "Immutable",
            Self::ImmutableBlockRollback(_) => "Immutable Rollback",
            Self::Block(_) => "Live",
            Self::BlockTip(_) => "Tip",
            Self::Rollback(_) => "Rollback",
        };

        let decoded_block = self.block_data().decode();
        match decoded_block {
            Err(err) => {
                write!(f, "{block_type} : failed to decode block: {err}")?;
            },
            Ok(block) => {
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
            },
        }
        Ok(())
    }
}

impl ChainUpdate {
    /// Gets the chain update's block data.
    #[must_use]
    pub fn block_data(&self) -> &MultiEraBlockData {
        match self {
            ChainUpdate::ImmutableBlock(block_data)
            | ChainUpdate::ImmutableBlockRollback(block_data)
            | ChainUpdate::Block(block_data)
            | ChainUpdate::BlockTip(block_data)
            | ChainUpdate::Rollback(block_data) => block_data,
        }
    }
}
