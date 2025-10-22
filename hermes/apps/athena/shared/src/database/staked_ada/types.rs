use cardano_blockchain_types::{
    hashes::TransactionId,
    pallas_addresses::StakeAddress,
    pallas_primitives::{AssetName, BigInt, PolicyId},
};
use derive_more::From;

/// Get UTXO assets query data.
#[derive(From)]
pub struct TxoByStakeRow {
    /// 29 Byte stake hash (CIP19).
    pub staked_address: StakeAddress,
    /// TXO transaction hash.
    pub txn_id: TransactionId,
    /// TXO transaction index within the slot.
    pub txn_index: u16,
    /// TXO index.
    pub txo: u16,
    /// TXO transaction slot number.
    pub slot_no: u64,
    /// TXO value.
    pub value: BigInt,
    /// TXO spent slot.
    pub spent_slot: Option<u64>,
}

/// [`TxoByStakeRow`] represented by a tuple.
pub(super) type TxoByStakeRowTuple = (
    StakeAddress,
    TransactionId,
    u16,
    u16,
    u64,
    BigInt,
    Option<u64>,
);

/// UTXO assets query row.
#[derive(From)]
pub struct TxoAssetsByStakeRow {
    /// 29 Byte stake hash (CIP19).
    pub stake_address: StakeAddress,
    /// TXO transaction index within the slot.
    pub txn_index: u16,
    /// TXO index.
    pub txo: u16,
    /// TXO transaction slot number.
    pub slot_no: u64,
    /// Asset policy hash (28 bytes).
    pub policy_id: PolicyId,
    /// Asset name (range of 0 - 32 bytes)
    pub asset_name: AssetName,
    /// Asset value.
    pub value: BigInt,
}

/// [`TxoAssetsByStakeRow`] represented by a tuple.
pub(super) type TxoAssetsByStakeRowTuple =
    (StakeAddress, u16, u16, u64, PolicyId, AssetName, BigInt);

/// TXI query data.
#[derive(From)]
pub struct TxiByTxnIdRow {
    /// TXI transaction hash.
    pub txn_id: TransactionId,
    /// TXI original TXO index.
    pub txo: u16,
    /// TXI slot number.
    pub slot_no: u64,
}

/// [`TxiByTxnIdRow`] represented by a tuple.
pub(super) type TxiByTxnIdsRowTuple = (TransactionId, u16, u64);

/// Update TXO spent query params.
pub struct UpdateTxoSpentParams {
    /// TXO stake address.
    pub stake_address: StakeAddress,
    /// TXO transaction index within the slot.
    pub txn_index: u16,
    /// TXO index.
    pub txo: u16,
    /// TXO slot number.
    pub slot_no: u64,
    /// TXO spent slot number.
    pub spent_slot: u64,
}
