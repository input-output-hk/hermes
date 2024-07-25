//! Indexes we maintain so that its faster to reference stuff by hash, etc.
//!
//! Cardano often uses block or transaction hashes to reference another entity.
//! These can not be found without either keeping an index of them, or doing
//! exhaustive searches.
//!
//! Typically these indexes are put into a shared DB, which can significantly slow down
//! the process of updating data when it references another entity.
//!
//! The aim here is to keep the index locally for maximum possible performance.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use anyhow::bail;
use crossbeam_skiplist::{SkipMap, SkipSet};
use heed::EnvOpenOptions;
use pallas_crypto::hash::Hash;
use tokio::sync::mpsc::{self, Sender};
use tracing::{debug, error};

use crate::{mithril_snapshot_config::MithrilSnapshotConfig, MultiEraBlock, Network};

type Key = heed::types::Bytes;
type Value = heed::types::Bytes;

/// The name of the Index Database file
const INDEX_DB_NAME: &str = "index.mdb";

/// The name of the Block Index Database file
const BLOCK_INDEX_TABLE_NAME: &str = "block_index";
/// The name of the Transaction Index Database file
const TRANSACTION_INDEX_TABLE_NAME: &str = "transaction_index";

/// The Block Index Table
type BlockIndexTable = heed::Database<Key, Value>;

/// The Transaction Index Table
type TransactionIndexTable = heed::Database<Key, Value>;

/// The Database Type we store in the DB.
type KvDb = (heed::Env, Arc<SkipMap<[u8; 32], (u64, u16)>>);

/// The Mainnet Index DB
static MAINNET_INDEX_DB: OnceLock<KvDb> = OnceLock::new();
/// The Preprod Index DB
static PREPROD_INDEX_DB: OnceLock<KvDb> = OnceLock::new();
/// The Preview Index DB
static PREVIEW_INDEX_DB: OnceLock<KvDb> = OnceLock::new();

/// Size of the Index DB in-Memory Cache.
// const DB_CACHE_SIZE: usize = 0x4000_0000; // 1GB

/// Initialize the Index DB
pub(crate) fn init_index_db(cfg: &MithrilSnapshotConfig) -> anyhow::Result<()> {
    let mut db_path = cfg.db_path();
    db_path.push(INDEX_DB_NAME);
    fs::create_dir_all(db_path.clone())?;

    let env = unsafe {
        EnvOpenOptions::new().
        map_size(100 * 1024 * 1024 * 1024).    // 100 GB
        max_dbs(2).open(db_path)?
    };

    let mut wtxn = env.write_txn()?;

    let _block_db: BlockIndexTable =
        env.create_database(&mut wtxn, Some(BLOCK_INDEX_TABLE_NAME))?;
    let _transaction_dn: TransactionIndexTable =
        env.create_database(&mut wtxn, Some(TRANSACTION_INDEX_TABLE_NAME))?;

    wtxn.commit()?;

    env.force_sync()?;

    // Create the in-memory index for the live chain
    let in_memory_db: Arc<SkipMap<[u8; 32], (u64, u16)>> = Arc::new(SkipMap::new());

    let already_initialized = match cfg.chain {
        Network::Mainnet => MAINNET_INDEX_DB.set((env, in_memory_db)),
        Network::Preprod => PREPROD_INDEX_DB.set((env, in_memory_db)),
        Network::Preview => PREVIEW_INDEX_DB.set((env, in_memory_db)),
    };

    if already_initialized.is_err() {
        // Should not happen.
        bail!("Index DB already initialized");
    }

    Ok(())
}

/// Get the Database for a given chain.
fn get_db(chain: Network) -> Option<&'static KvDb> {
    match chain {
        Network::Mainnet => MAINNET_INDEX_DB.get(),
        Network::Preprod => PREPROD_INDEX_DB.get(),
        Network::Preview => PREVIEW_INDEX_DB.get(),
    }
}

/// The Write Transaction we create when updating the indexes.
#[derive(Clone)]
pub(crate) struct DBWriteTransaction(Sender<DBWriteUpdate>);

impl DBWriteTransaction {
    /// Create a new Write Transaction for the Blockchain Index.
    pub(crate) fn new(chain: Network) -> anyhow::Result<DBWriteTransaction> {
        let (tx, mut rx) = mpsc::channel::<DBWriteUpdate>(DB_UPDATE_QUEUE_SIZE);

        tokio::task::spawn_blocking(move || {
            let Some((db, _)) = get_db(chain) else {
                error!("DB not initialized");
                return;
            };

            let mut wtxn = match db.write_txn() {
                Ok(wtxn) => wtxn,
                Err(error) => {
                    error!(error=%error, "Index DB Write Transaction Error");
                    return;
                },
            };

            let block_table: BlockIndexTable =
                match db.create_database(&mut wtxn, Some(BLOCK_INDEX_TABLE_NAME)) {
                    Ok(block_table) => block_table,
                    Err(error) => {
                        error!(error=%error, "Index DB Block Table Error");
                        return;
                    },
                };

            let transaction_table: TransactionIndexTable =
                match db.create_database(&mut wtxn, Some(TRANSACTION_INDEX_TABLE_NAME)) {
                    Ok(transaction_table) => transaction_table,
                    Err(error) => {
                        error!(error=%error, "Index DB Transaction Table Error");
                        return;
                    },
                };

            let mut blocks_indexed: u64 = 0;
            let mut txn_indexed: u64 = 0;

            loop {
                match rx.blocking_recv() {
                    Some(DBWriteUpdate::BlockHash(hash, slot_no)) => {
                        let key: [u8; 32] = *hash;
                        let value = serialize_value(slot_no, 0);
                        if let Err(error) = block_table.put_with_flags(
                            &mut wtxn,
                            heed::PutFlags::NO_OVERWRITE,
                            &key,
                            &value,
                        ) {
                            error!(chain=%chain, error=%error, "Error while writing Block Hash to index db: {}:{}",hash, slot_no);
                            return;
                        }
                        blocks_indexed += 1;
                        if blocks_indexed % 100_000 == 0 {
                            debug!("indexed {blocks_indexed} blocks");
                        }
                    },
                    Some(DBWriteUpdate::TransactionHash(hash, slot_no, txn_index)) => {
                        let key: [u8; 32] = *hash;
                        let value = serialize_value(slot_no, txn_index);
                        if let Err(error) = transaction_table.put_with_flags(
                            &mut wtxn,
                            heed::PutFlags::NO_OVERWRITE,
                            &key,
                            &value,
                        ) {
                            error!(chain=%chain, error=%error, "Error while writing Transaction Hash to index db: {}:{}/{}",hash, slot_no, txn_index);
                            return;
                        }
                        txn_indexed += 1;
                        if txn_indexed % 100_000 == 0 {
                            debug!("indexed {txn_indexed} transactions");
                        }
                    },
                    Some(DBWriteUpdate::Commit) => {
                        if let Err(error) = wtxn.commit() {
                            error!(chain=%chain, error=%error,"Error while committing index db");
                            return;
                        }
                        break;
                    },
                    Some(DBWriteUpdate::Rollback) | None => {
                        wtxn.abort();
                        break;
                    },
                }
            }

            debug!("Done: indexed {blocks_indexed} blocks");
            debug!("Done: indexed {txn_indexed} traactions");

            if let Err(error) = db.force_sync() {
                error!(error=%error,"Error syncing index db");
            }
        });

        Ok(DBWriteTransaction(tx))
    }

    /// Add a Block hash to the index db.
    pub(crate) fn index_block_hash(&self, hash: &Hash<32>, slot_no: u64) -> anyhow::Result<()> {
        self.0
            .blocking_send(DBWriteUpdate::BlockHash(*hash, slot_no))?;

        Ok(())
    }

    /// Add a Transaction hash to the index db.
    pub(crate) fn index_transaction_hash(
        &self, hash: &Hash<32>, slot_no: u64, txn_offset: u16,
    ) -> anyhow::Result<()> {
        self.0
            .blocking_send(DBWriteUpdate::TransactionHash(*hash, slot_no, txn_offset))?;

        Ok(())
    }

    /// Commit the transaction.
    pub(crate) async fn commit(&self) -> anyhow::Result<()> {
        self.0.send(DBWriteUpdate::Commit).await?;

        Ok(())
    }

    /// Rollback the transaction.
    pub(crate) async fn rollback(&self) -> anyhow::Result<()> {
        self.0.send(DBWriteUpdate::Rollback).await?;

        Ok(())
    }
}

/// DB Updates are preformed in their own thread, so that there is a single writer
/// that controls the DB and all the updates for a single transaction can come from
/// multiple data producers.
pub(crate) enum DBWriteUpdate {
    /// Add a block hash to the index db.
    BlockHash(Hash<32>, u64),
    /// Add a transaction hash to the index db.
    TransactionHash(Hash<32>, u64, u16),
    /// Commit the transaction.
    Commit,
    /// Rollback the transaction.
    Rollback,
}

/// How many Index DB updates we can have pending.
const DB_UPDATE_QUEUE_SIZE: usize = 1024;

/// Internal Index struct we use to serialize the index values
#[derive(minicbor::Encode, minicbor::Decode, minicbor::CborLen, Clone, Debug)]
#[cbor(array)]
struct HashIndex {
    /// The Slot Number the hash is found in.
    #[n(0)]
    slot_no: u64,

    /// For a transaction, the transaction offset it is.  For a block hash, always 0.
    #[n(1)]
    txn_offset: u16,
}

/// Super simple serialize our slot number and transaction offset to a 10 byte array.
///
/// Note, for Block hash, just set the `txn_offset` to 0.
fn serialize_value(slot_no: u64, txn_offset: u16) -> Vec<u8> {
    let to_encode = HashIndex {
        slot_no,
        txn_offset,
    };

    if let Ok(bytes) = minicbor::to_vec(to_encode) {
        bytes
    } else {
        error!("Failed to serialize the index value. Can't Happen.");
        Vec::new()
    }
}

/// Super simple serialize our slot number and transaction offset to a 10 byte array.
///
/// For a Block hash, the transaction offset will always be 0.
/// Therefore its not possible to tell if a hash is a block hash or a transaction hash.
/// But they should never collide due to the properties of the hash.
#[allow(dead_code)]
fn deserialize_value(value: [u8; 10]) -> (u64, u16) {
    // It is 100% safe to use unwrap() here as it can never be anything but 8 bytes.
    #[allow(clippy::unwrap_used)]
    let slot_bytes: [u8; 8] = value[0..8].try_into().unwrap();
    let slot = u64::from_be_bytes(slot_bytes);

    // It is 100% safe to use unwrap() here as it can never be anything but 8 bytes.
    #[allow(clippy::unwrap_used)]
    let txn_offset_bytes: [u8; 2] = value[8..].try_into().unwrap();
    let txn_offset = u16::from_be_bytes(txn_offset_bytes);

    (slot, txn_offset)
}

/// Index an immutable chunk into the On-Disk Indexes.
pub(crate) fn index_immutable_chunk(
    chain: Network, chunk_path: &Path, transaction: &DBWriteTransaction,
) {
    // debug!("Indexing chunk: {:?}", chunk_path);

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
                                let block_hash = decoded_block.hash();
                                let block_slot = decoded_block.slot();

                                if let Err(error) =
                                    transaction.index_block_hash(&block_hash, block_slot)
                                {
                                    error!(chain=%chain, error=%error, "Error indexing block hash: {}:{}", block_hash, block_slot);
                                    return;
                                };

                                // debug!(
                                //    chain = %chain,
                                //    "Indexing block: {} @ {}", block_hash, block_slot
                                //);

                                for (txn_offset, txn) in decoded_block.txs().into_iter().enumerate()
                                {
                                    let offset = u16::try_from(txn_offset).unwrap_or(u16::MAX);
                                    let txn_hash = txn.hash();

                                    if let Err(error) = transaction
                                        .index_transaction_hash(&txn_hash, block_slot, offset)
                                    {
                                        error!(chain=%chain, error=%error, "Error indexing transaction hash: {}:{}", block_hash, block_slot);
                                        return;
                                    };
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
            return;
        },
    }
}

/// Add a live block to the in-memory block index
pub(crate) fn index_live_block(chain: Network, block: &MultiEraBlock) {
    let decoded_block = block.decode();
    let block_hash: [u8; 32] = *decoded_block.hash();
    let block_slot = decoded_block.slot();

    let Some((_, in_memory_db)) = get_db(chain) else {
        error!("Failed to get the in-memory index db. Should not happen.");
        return;
    };

    // Save the block index.
    let _unused = in_memory_db.insert(block_hash, (block_slot, 0));

    for (txn_offset, txn) in decoded_block.txs().into_iter().enumerate() {
        let offset = u16::try_from(txn_offset).unwrap_or(u16::MAX);
        let txn_hash: [u8; 32] = *txn.hash();

        // Index the transactions within the block.
        let _unused = in_memory_db.insert(txn_hash, (block_slot, offset));
    }
}

/// Purges all data from the live block index thats < `max_slot`.
pub(crate) fn purge_index_live_block(chain: Network, max_slot: u64) {
    let Some((_, in_memory_db)) = get_db(chain) else {
        error!("Failed to get the in-memory index db. Should not happen.");
        return;
    };

    for entry in in_memory_db.iter() {
        let slot = entry.value().0;
        if slot < max_slot {
            let _unused = entry.remove();
        }
    }
}

/// In the background, index all the blocks and transactions that updated.
pub(crate) fn background_index_blocks_and_transactions(
    chain: Network, validation_handle: tokio::task::JoinHandle<bool>,
    chunk_list: Arc<SkipSet<PathBuf>>,
) -> tokio::task::JoinHandle<bool> {
    tokio::spawn(async move {
        debug!(
            "Index Blocks and Transactions background updater for: {} : Started",
            chain
        );

        let txn = match DBWriteTransaction::new(chain) {
            Ok(txn) => txn,
            Err(error) => {
                error!(chain=%chain, error=%error, "Failed to get Write Transaction for DB");
                return false;
            },
        };

        let inner_txn = txn.clone();
        let inner_indexer_handle = tokio::task::spawn_blocking(move || {
            rayon::scope(|s| {
                chunk_list.iter().for_each(|chunk| {
                    let txn = inner_txn.clone();
                    let chunk_value = chunk.value().clone();
                    s.spawn(move |_| {
                        // task s.1
                        index_immutable_chunk(chain, &chunk_value, &txn);
                    });
                });
            });
            debug!("Finished Indexing.");
        });
        let _unused = inner_indexer_handle.await;

        debug!(
            "Index Blocks and Transactions background updater for: {} : Finished",
            chain
        );

        let mithril_valid = validation_handle.await.unwrap_or(false);

        if mithril_valid {
            // Commit transaction
            if let Err(error) = txn.commit().await {
                error!(chain=%chain, error=%error, "Failed to commit Write Transaction for DB");
                return false;
            }
        } else {
            // Rollback
            if let Err(error) = txn.rollback().await {
                error!(chain=%chain, error=%error, "Failed to rollback Write Transaction for DB");
                return false;
            }
        }

        mithril_valid
    })
}
