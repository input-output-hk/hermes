//! Cardano block functions

use cardano_blockchain_types::MultiEraBlock;
use serde_json::json;

use crate::{
    hermes::{
        self,
        hermes::cardano::api::{Block, CardanoNetwork},
    },
    utils::log::log_error,
};

/// Build a `cardano_blockchain_types` `MultiEraBlock`
/// 
/// # Return
/// 
/// * `Option<MultiEraBlock>` - A `MultiEraBlock` if successful, `None` otherwise
pub(crate) fn build_block(
    file_name: &str,
    func_name: &str,
    network: CardanoNetwork,
    block_resource: &Block,
) -> Option<MultiEraBlock> {
    // Create a pallas block from a raw block data
    let raw_block = block_resource.raw();
    let pallas_block =
        match cardano_blockchain_types::pallas_traverse::MultiEraBlock::decode(&raw_block) {
            Ok(block) => block,
            Err(_) => {
                log_error(
                    file_name,
                    func_name,
                    "pallas_traverse::MultiEraBlock::decode",
                    "Failed to decode pallas block from raw block data",
                    None,
                );
                return None;
            },
        };

    let prv_slot = match pallas_block.slot().checked_sub(1) {
        Some(slot) => slot,
        None => {
            log_error(
                file_name,
                func_name,
                "pallas_block.slot().checked_sub()",
                "Slot underflow when computing previous point",
                Some(&json!({ "slot": pallas_block.slot() }).to_string()),
            );
            return None;
        },
    };

    let prv_hash = match pallas_block.header().previous_hash() {
        Some(hash) => hash,
        None => {
            log_error(
                file_name,
                func_name,
                "pallas_block.header().previous_hash()",
                "Missing previous hash in block header",
                None,
            );
            return None;
        },
    };

    // Need previous point in order to construct our multi-era block
    let prv_point = cardano_blockchain_types::Point::new(prv_slot.into(), prv_hash.into());

    // Construct our version of multi-era block
    let block = match cardano_blockchain_types::MultiEraBlock::new(
        network.into(),
        raw_block.clone(),
        &prv_point,
        block_resource.get_fork().into(),
    ) {
        Ok(block) => block,
        Err(_) => {
            log_error(
                file_name,
                func_name,
                "cardano_blockchain_types::MultiEraBlock::new",
                "Failed to construct multi-era block",
                None,
            );
            return None;
        },
    };

    Some(block)
}
