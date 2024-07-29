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

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock, RwLock},
};

use anyhow::bail;
use crossbeam_skiplist::SkipSet;
use pallas_crypto::hash::Hash;
use redb::{Database, TableDefinition};
use tokio::sync::mpsc::{self, Sender};
use tracing::{debug, error};

use crate::{mithril_snapshot_config::MithrilSnapshotConfig, Network};

/// The name of the Index Database file
const INDEX_DB_NAME: &str = "index.redb";

/// The name of the Block Index Database file
const BLOCK_INDEX_TABLE_NAME: &str = "block_index";
/// The name of the Transaction Index Database file
const TRANSACTION_INDEX_TABLE_NAME: &str = "transaction_index";

/// The Block Index Table
const BLOCK_INDEX_TABLE: TableDefinition<[u8; 32], Vec<u8>> =
    TableDefinition::new(BLOCK_INDEX_TABLE_NAME);

/// The Transaction Index Table
const TRANSACTION_INDEX_TABLE: TableDefinition<[u8; 32], Vec<u8>> =
    TableDefinition::new(TRANSACTION_INDEX_TABLE_NAME);

/// The Database Type we store in the DB.
type KvDb = Arc<RwLock<Database>>;

/// The Mainnet Index DB
static MAINNET_INDEX_DB: OnceLock<KvDb> = OnceLock::new();
/// The Preprod Index DB
static PREPROD_INDEX_DB: OnceLock<KvDb> = OnceLock::new();
/// The Preview Index DB
static PREVIEW_INDEX_DB: OnceLock<KvDb> = OnceLock::new();

/// Size of the Index DB in-Memory Cache.
const DB_CACHE_SIZE: usize = 0x4000_0000; // 1GB

/// Initialize the Index DB
pub(crate) fn init_index_db(cfg: &MithrilSnapshotConfig) -> anyhow::Result<()> {
    let mut db_path = cfg.db_path();
    fs::create_dir_all(db_path.clone())?;
    db_path.push(INDEX_DB_NAME);

    let db = Arc::new(RwLock::new(
        redb::Builder::new()
            .set_cache_size(DB_CACHE_SIZE)
            .create(db_path)?,
    ));

    let already_initialized = match cfg.chain {
        Network::Mainnet => MAINNET_INDEX_DB.set(db),
        Network::Preprod => PREPROD_INDEX_DB.set(db),
        Network::Preview => PREVIEW_INDEX_DB.set(db),
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
        let Some(db) = get_db(chain) else {
            bail!("DB not initialized");
        };

        let (tx, mut rx) = mpsc::channel::<DBWriteUpdate>(DB_UPDATE_QUEUE_SIZE);

        tokio::task::spawn_blocking(move || {
            let mut db = match db.write() {
                Ok(db) => db,
                Err(error) => {
                    error!(error=%error, "Somehow the Database RwLock is Poisoned. Should not happen.");
                    return;
                },
            };

            //let mut txn = db.begin_with_mode(Mode::WriteOnly)?;
            let mut txn = match db.begin_write() {
                Ok(txn) => txn,
                Err(error) => {
                    error!(error=%error, "Index DB Write Transaction Error");
                    return;
                },
            };
            txn.set_durability(redb::Durability::Immediate);

            let mut block_table = match txn.open_table(BLOCK_INDEX_TABLE) {
                Ok(block_table) => block_table,
                Err(error) => {
                    error!(error=%error, "Index DB Block Table Error");
                    return;
                },
            };

            let mut transaction_table = match txn.open_table(TRANSACTION_INDEX_TABLE) {
                Ok(transaction_table) => transaction_table,
                Err(error) => {
                    error!(error=%error, "Index DB Transaction Table Error");
                    return;
                },
            };

            loop {
                match rx.blocking_recv() {
                    Some(DBWriteUpdate::BlockHash(hash, slot_no)) => {
                        let key: [u8; 32] = *hash;
                        let value = serialize_value(slot_no, 0);
                        if let Err(error) = block_table.insert(key, &value) {
                            error!(chain=%chain, error=%error, "Error while writing Block Hash to index db: {}:{}",hash, slot_no);
                        }
                    },
                    Some(DBWriteUpdate::TransactionHash(hash, slot_no, txn_index)) => {
                        let key: [u8; 32] = *hash;
                        let value = serialize_value(slot_no, txn_index);
                        if let Err(error) = transaction_table.insert(key, &value) {
                            error!(chain=%chain, error=%error, "Error while writing Transaction Hash to index db: {}:{}/{}",hash, slot_no, txn_index);
                        }
                    },
                    Some(DBWriteUpdate::Commit) => {
                        drop(block_table);
                        drop(transaction_table);
                        if let Err(error) = txn.commit() {
                            error!(chain=%chain, error=%error,"Error while committing index db");
                        }
                        break;
                    },
                    Some(DBWriteUpdate::Rollback) | None => {
                        drop(block_table);
                        drop(transaction_table);
                        if let Err(error) = txn.abort() {
                            error!(chain=%chain, error=%error,"Error while rolling back index db");
                        }
                        break;
                    },
                }
            }

            if let Err(error) = db.compact() {
                error!(error=%error,"Error compacting index db");
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
                                let block_hash = decoded_block.hash();
                                let block_slot = decoded_block.slot();

                                if let Err(error) =
                                    transaction.index_block_hash(&block_hash, block_slot)
                                {
                                    error!(chain=%chain, error=%error, "Error indexing block hash: {}:{}", block_hash, block_slot);
                                };

                                //debug!(
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
                                        error!(chain=%chain, error=%error, "Error indexing block hash: {}:{}", block_hash, block_slot);
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
        },
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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Ok;
    use minicbor::decode;

    #[test]
    fn test_serialize_value_simple() {
        let slot_no = 12345;
        let txn_offset = 678;
        
        let serialized = serialize_value(slot_no, txn_offset);
        
        assert!(!serialized.is_empty(), "Serialized output must not be empty");

        let decoded: HashIndex = decode(&serialized).expect("Failed to decode serialized bytes");

        assert_eq!(decoded.slot_no, slot_no);
        assert_eq!(decoded.txn_offset, txn_offset);
    }

    #[test]
    fn test_deserialize_value_simple() {
        let slot_no = 12345u64;
        let txn_offset = 678u16;

        let mut value = [0u8; 10];
        value[0..8].copy_from_slice(&slot_no.to_be_bytes());
        value[8..10].copy_from_slice(&txn_offset.to_be_bytes());

        let (deserialized_slot_no, deserialized_txn_offset) = deserialize_value(value);

        assert_eq!(deserialized_slot_no, slot_no);
        assert_eq!(deserialized_txn_offset, txn_offset);
    }

    #[tokio::test]
    async fn test_index_db_simple() -> anyhow::Result<()> {
        init_index_db(&MithrilSnapshotConfig::default_for(Network::Preprod))?;
        let db_write_transaction = DBWriteTransaction::new(Network::Preprod)?;

        let hash = Hash::new([1; 32]);
        let slot_no = 42;
        let txn_offset = 7;

        db_write_transaction.commit().await.expect("cannot commit");

        db_write_transaction.rollback().await.expect("cannot rollback");

        tokio::task::spawn_blocking(move || {
            // calling this function without wrapping it inside `spawn_blocking` will cause thread panic
            db_write_transaction.index_block_hash(&hash, slot_no).expect("cannot index block hash");

            db_write_transaction.index_transaction_hash(&hash, slot_no, txn_offset).expect("cannot index transaction hash")
        });

        Ok(())
    }
}
