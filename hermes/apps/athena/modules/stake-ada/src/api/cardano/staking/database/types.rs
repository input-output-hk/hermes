use std::sync::{Arc, RwLock};

use anyhow::{bail, Context};
use cardano_blockchain_types::{hashes::TransactionId, Slot, StakeAddress, TxnIndex};
use shared::utils::sqlite;

/// A `TransactionHash` wrapper that can be stored to and load from a database.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DbTransactionId(TransactionId);

impl TryFrom<sqlite::Value> for DbTransactionId {
    type Error = anyhow::Error;

    fn try_from(value: sqlite::Value) -> Result<Self, Self::Error> {
        match value {
            sqlite::Value::Blob(bytes) => Ok(TransactionId::try_from(bytes)
                .with_context(|| "Failed to decode TransactionId")?
                .into()),
            _ => bail!("Invalid value type selected"),
        }
    }
}

/// A `TxnIndex` wrapper that can be stored to and load from a database.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DbTxnIndex(i16);

impl TryFrom<sqlite::Value> for DbTxnIndex {
    type Error = anyhow::Error;

    fn try_from(value: sqlite::Value) -> Result<Self, Self::Error> {
        match value {
            sqlite::Value::Int32(txo) => Ok(DbTxnIndex(
                txo.try_into()
                    .with_context(|| "Transaction index not in i16 range")?,
            )),
            sqlite::Value::Int64(txo) => Ok(DbTxnIndex(
                txo.try_into()
                    .with_context(|| "Transaction index not in i16 range")?,
            )),
            _ => bail!("Invalid value type selected"),
        }
    }
}

/// A `TxnOutputOffset` wrapper that can be stored to and load from a database.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DbTxnOutputOffset(i16);

impl TryFrom<sqlite::Value> for DbTxnOutputOffset {
    type Error = anyhow::Error;

    fn try_from(value: sqlite::Value) -> Result<Self, Self::Error> {
        match value {
            sqlite::Value::Int32(txo) => Ok(DbTxnOutputOffset(
                txo.try_into().with_context(|| "Txo not in i16 range")?,
            )),
            sqlite::Value::Int64(txo) => Ok(DbTxnOutputOffset(
                txo.try_into().with_context(|| "Txo not in i16 range")?,
            )),
            _ => bail!("Invalid value type selected"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DbSlot(u64);

impl TryFrom<sqlite::Value> for DbSlot {
    type Error = anyhow::Error;

    fn try_from(value: sqlite::Value) -> Result<Self, Self::Error> {
        match value {
            sqlite::Value::Int32(txo) => Ok(DbSlot(
                txo.try_into().with_context(|| "Slot not in u64 range")?,
            )),
            sqlite::Value::Int64(txo) => Ok(DbSlot(
                txo.try_into().with_context(|| "Slot not in u64 range")?,
            )),
            _ => bail!("Invalid value type selected"),
        }
    }
}

impl TryFrom<DbSlot> for sqlite::Value {
    type Error = anyhow::Error;

    fn try_from(value: DbSlot) -> Result<Self, Self::Error> {
        Ok(sqlite::Value::Int64(value.0.try_into()?))
    }
}

pub struct DbValue(num_bigint::BigInt);

impl TryFrom<sqlite::Value> for DbValue {
    type Error = anyhow::Error;

    fn try_from(value: sqlite::Value) -> Result<Self, Self::Error> {
        match value {
            // TODO: fix according to stored value.
            sqlite::Value::Blob(bytes) => {
                Ok(DbValue(num_bigint::BigInt::from_signed_bytes_be(&bytes)))
            },
            _ => bail!("Invalid value type selected"),
        }
    }
}

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

/// Get native assets query.
#[derive(Clone)]
pub(crate) struct GetAssetsByStakeAddressQuery {
    /// Key Data.
    pub key: Arc<GetAssetsByStakeAddressQueryKey>,
    /// Value Data.
    pub value: Arc<GetAssetsByStakeAddressQueryValue>,
}

impl From<DbStakeAddress> for sqlite::Value {
    fn from(value: DbStakeAddress) -> Self {
        Self::Blob(value.0.into())
    }
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

impl From<DbTransactionId> for sqlite::Value {
    fn from(value: DbTransactionId) -> Self {
        sqlite::Value::Blob(value.0.into())
    }
}

impl From<DbValue> for num_bigint::BigInt {
    fn from(value: DbValue) -> Self {
        value.0
    }
}

impl From<DbTxnIndex> for sqlite::Value {
    fn from(value: DbTxnIndex) -> Self {
        Self::Int64(value.0.into())
    }
}

impl From<DbTxnOutputOffset> for sqlite::Value {
    fn from(value: DbTxnOutputOffset) -> Self {
        Self::Int64(value.0.into())
    }
}
