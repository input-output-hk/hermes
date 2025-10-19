use std::sync::{Arc, RwLock};

use cardano_blockchain_types::{hashes::TransactionId, Slot, StakeAddress, TxnIndex};
use shared::bindings::hermes::sqlite::api::Sqlite;

/// A `TransactionHash` wrapper that can be stored to and load from a database.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DbTransactionId(TransactionId);

/// A `TxnIndex` wrapper that can be stored to and load from a database.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DbTxnIndex(i16);

/// A `TxnOutputOffset` wrapper that can be stored to and load from a database.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DbTxnOutputOffset(i16);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DbSlot(u64);

/// Get TXI query.
pub(crate) struct GetTxiByTxnHashesQuery {
    /// TXI transaction hash.
    pub txn_id: DbTransactionId,
    /// TXI original TXO index.
    pub txo: DbTxnOutputOffset,
    /// TXI slot number.
    pub slot_no: DbSlot,
}

/// Get UTXO assets query key.
#[derive(Hash, PartialEq, Eq, Debug)]
pub(crate) struct GetTxoByStakeAddressQueryKey {
    /// TXO transaction index within the slot.
    pub txn_index: DbTxnIndex,
    /// TXO index.
    pub txo: DbTxnOutputOffset,
    /// TXO transaction slot number.
    pub slot_no: DbSlot,
}

/// Get native assets query.
#[derive(Hash, PartialEq, Eq, Debug)]
pub(crate) struct GetAssetsByStakeAddressQueryKey {
    /// TXO transaction index within the slot.
    pub txn_index: DbTxnIndex,
    /// TXO index.
    pub txo: DbTxnOutputOffset,
    /// TXO transaction slot number.
    pub slot_no: DbSlot,
}

/// Get native assets query.
pub(crate) struct GetAssetsByStakeAddressQueryValue {
    /// Asset policy hash (28 bytes).
    pub policy_id: Vec<u8>,
    /// Asset name (range of 0 - 32 bytes)
    pub asset_name: Vec<u8>,
    /// Asset value.
    pub value: num_bigint::BigInt,
}

/// Get UTXO assets query.
#[derive(Clone)]
pub(crate) struct GetTxoByStakeAddressQuery {
    /// Key Data.
    pub key: Arc<GetTxoByStakeAddressQueryKey>,
    /// Value Data.
    pub value: Arc<RwLock<GetTxoByStakeAddressQueryValue>>,
}

/// Get UTXO assets query value.
pub(crate) struct GetTxoByStakeAddressQueryValue {
    /// TXO transaction hash.
    pub txn_id: TransactionId,
    /// TXO value.
    pub value: num_bigint::BigInt,
    /// TXO spent slot.
    pub spent_slot: Option<DbSlot>,
}

/// A binary `CIP-19` stack address (29  bytes) that can be stored to and load from a
/// database.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct DbStakeAddress(StakeAddress);

/// Update TXO spent query params.
#[derive(Clone, Debug)]
pub(crate) struct UpdateTxoSpentQueryParams {
    /// TXO stake address.
    pub stake_address: DbStakeAddress,
    /// TXO transaction index within the slot.
    pub txn_index: DbTxnIndex,
    /// TXO index.
    pub txo: DbTxnOutputOffset,
    /// TXO slot number.
    pub slot_no: DbSlot,
    /// TXO spent slot number.
    pub spent_slot: DbSlot,
}

impl From<DbTxnIndex> for TxnIndex {
    fn from(val: DbTxnIndex) -> Self {
        val.0.into()
    }
}

impl From<DbTxnOutputOffset> for i16 {
    fn from(val: DbTxnOutputOffset) -> Self {
        val.0
    }
}

impl From<DbSlot> for Slot {
    fn from(val: DbSlot) -> Self {
        val.0.into()
    }
}

impl From<StakeAddress> for DbStakeAddress {
    fn from(val: StakeAddress) -> Self {
        DbStakeAddress(val)
    }
}

impl From<TxnIndex> for DbTxnIndex {
    fn from(val: TxnIndex) -> Self {
        DbTxnIndex(val.into())
    }
}

impl From<i16> for DbTxnOutputOffset {
    fn from(val: i16) -> Self {
        DbTxnOutputOffset(val)
    }
}

impl From<Slot> for DbSlot {
    fn from(val: Slot) -> Self {
        DbSlot(val.into())
    }
}

impl From<DbTransactionId> for TransactionId {
    fn from(val: DbTransactionId) -> Self {
        val.0
    }
}

impl From<TransactionId> for DbTransactionId {
    fn from(value: TransactionId) -> Self {
        Self(value)
    }
}

pub(crate) fn get_txi_by_txn_hashes(
    _session: &Sqlite,
    _txn_ids: &[TransactionId],
) -> anyhow::Result<Vec<GetTxiByTxnHashesQuery>> {
    todo!()
}

pub(crate) fn get_txo_by_stake_address(
    _session: &Sqlite,
    _stake_address: &StakeAddress,
) -> anyhow::Result<Vec<GetTxoByStakeAddressQuery>> {
    todo!()
}

/// Get native assets query.
#[derive(Clone)]
pub(crate) struct GetAssetsByStakeAddressQuery {
    /// Key Data.
    pub key: Arc<GetAssetsByStakeAddressQueryKey>,
    /// Value Data.
    pub value: Arc<GetAssetsByStakeAddressQueryValue>,
}

pub(crate) fn get_assets_by_stake_address(
    _session: &Sqlite,
    _stake_address: &StakeAddress,
) -> anyhow::Result<Vec<GetAssetsByStakeAddressQuery>> {
    todo!()
}

pub(crate) fn update_txo_spent(
    _session: &Sqlite,
    _params: Vec<UpdateTxoSpentQueryParams>,
) -> anyhow::Result<()> {
    todo!()
}
