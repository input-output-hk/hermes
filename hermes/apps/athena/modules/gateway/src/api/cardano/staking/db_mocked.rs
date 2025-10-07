use std::sync::{Arc, RwLock};

use cardano_blockchain_types::{hashes::TransactionId, Slot, StakeAddress, TxnIndex};

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

impl Into<TxnIndex> for DbTxnIndex {
    fn into(self) -> TxnIndex {
        self.0.into()
    }
}

impl Into<i16> for DbTxnOutputOffset {
    fn into(self) -> i16 {
        self.0
    }
}

impl Into<Slot> for DbSlot {
    fn into(self) -> Slot {
        self.0.into()
    }
}

impl Into<DbStakeAddress> for StakeAddress {
    fn into(self) -> DbStakeAddress {
        DbStakeAddress(self.into())
    }
}

impl Into<DbTxnIndex> for TxnIndex {
    fn into(self) -> DbTxnIndex {
        DbTxnIndex(self.into())
    }
}

impl Into<DbTxnOutputOffset> for i16 {
    fn into(self) -> DbTxnOutputOffset {
        DbTxnOutputOffset(self)
    }
}

impl Into<DbSlot> for Slot {
    fn into(self) -> DbSlot {
        DbSlot(self.into())
    }
}

impl Into<TransactionId> for DbTransactionId {
    fn into(self) -> TransactionId {
        self.0
    }
}
