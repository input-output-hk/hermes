//! Indexes we maintain so that its faster to reference stuff by hash, etc.
//!
//! Cardano often uses block or transaction hashes to reference another entity.
//! These can not be found without either keeping an index of them, or doing
//! exhaustive searches.
//!
//! Typically these indexes are put into a shared DB, which can significantly slow down the
//! process of updating data when it references another entity.
//!
//! The aim here is to keep the index locally for maximum possible performance.

use std::path::Path;

use tracing::error;

use crate::Network;

/// Index an immutable chunk into the On-Disk Indexes.
pub(crate) fn index_immutable_chunk(chain: Network, chunk_path: &Path) {
    //debug!("Indexing chunk: {:?}", chunk_path);

    let dir = chunk_path.parent().unwrap_or(Path::new(""));
    let name = chunk_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut block_count = 0;

    match pallas_hardano::storage::immutable::chunk::read_blocks(dir, &name) {
        Ok(iterator) => {
            for block in iterator {
                match block {
                    Ok(block) => {
                        let decoded = pallas::ledger::traverse::MultiEraBlock::decode(&block);
                        match decoded {
                            Ok(decoded_block) => {
                                let _block_hash = decoded_block.hash();
                                let _block_slot = decoded_block.slot();

                                //debug!(
                                //    chain = %chain,
                                //    "Indexing block: {} @ {}", block_hash, block_slot
                                //);

                                for (_txn_offset, txn) in
                                    decoded_block.txs().into_iter().enumerate()
                                {
                                    let _txn_hash = txn.hash();
                                    //    debug!(
                                    //        chain = %chain,
                                    //        "Indexing Transaction: {} @ {}:{}",
                                    //        txn_hash, block_slot, txn_offset
                                    //    );
                                }
                            },
                            Err(error) => {
                                error!(
                                    chain = %chain,
                                    block = block_count, error=%error, "Error while decoding block from: {}", chunk_path.to_string_lossy());
                                return;
                            },
                        }
                    },
                    Err(error) => {
                        error!(
                            chain = %chain,
                            block = block_count, error=%error, "Error while iterating block from: {}", chunk_path.to_string_lossy());
                        return;
                    },
                };
                block_count += 1;
            }
        },
        Err(error) => {
            error!(
                chain = %chain,
                error = %error, "Failed to iterate {} for indexing.",chunk_path.to_string_lossy());
        },
    }
}
