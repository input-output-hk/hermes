use cardano_blockchain_types::{
    hashes::TransactionId,
    StakeAddress,
    pallas_primitives::{AssetName, BigInt, PolicyId},
};
use derive_more::From;

use crate::utils::sqlite;

/// Row from `stake_registration` table.
#[derive(From)]
pub struct StakeRegistrationRow {
    /// 29 Byte stake hash (CIP19).
    pub stake_address: StakeAddress,
    /// TXO transaction slot number.
    pub slot_no: u64,
    /// TXO transaction index within the slot.
    pub txn_index: u16,
    /// Is the address a script address.
    pub script: bool,
    /// True if the this transaction contains cardano stake registration cert.
    pub register: bool,
    /// True if the this transaction contains cardano stake deregistration cert.
    pub deregister: bool,
    /// True if the this transaction contains CIP36 registration.
    pub cip36: bool,
    /// Stake was delegated to this Pool address.
    /// Not present if delegation did not change.
    pub pool_delegation: bool,
}

impl TryFrom<StakeRegistrationRow> for [sqlite::Value; 8] {
    type Error = anyhow::Error;

    fn try_from(v: StakeRegistrationRow) -> Result<Self, Self::Error> {
        Ok([
            v.stake_address.into(),
            sqlite::Value::try_from(v.slot_no)?,
            v.txn_index.into(),
            v.script.into(),
            v.register.into(),
            v.deregister.into(),
            v.cip36.into(),
            v.pool_delegation.into(),
        ])
    }
}

/// Row from `txo_by_stake` table.
#[derive(From)]
pub struct TxoByStakeRow {
    /// 29 Byte stake hash (CIP19).
    pub stake_address: StakeAddress,
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

impl TryFrom<TxoByStakeRow> for [sqlite::Value; 7] {
    type Error = anyhow::Error;

    fn try_from(v: TxoByStakeRow) -> Result<Self, Self::Error> {
        Ok([
            v.stake_address.into(),
            v.txn_id.into(),
            v.txn_index.into(),
            v.txo.into(),
            sqlite::Value::try_from(v.slot_no)?,
            sqlite::Value::try_from(v.value)?,
            v.spent_slot
                .map_or(Ok(sqlite::Value::Null), sqlite::Value::try_from)?,
        ])
    }
}

/// Row from `txo_assets_by_stake` table.
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

impl TryFrom<TxoAssetsByStakeRow> for [sqlite::Value; 7] {
    type Error = anyhow::Error;

    fn try_from(v: TxoAssetsByStakeRow) -> Result<Self, Self::Error> {
        Ok([
            v.stake_address.into(),
            v.txn_index.into(),
            v.txo.into(),
            sqlite::Value::try_from(v.slot_no)?,
            sqlite::Value::try_from(v.policy_id)?,
            v.asset_name.into(),
            sqlite::Value::try_from(v.value)?,
        ])
    }
}

/// Row from `txi_by_txn_id` table.
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

impl TryFrom<TxiByTxnIdRow> for [sqlite::Value; 3] {
    type Error = anyhow::Error;

    fn try_from(v: TxiByTxnIdRow) -> Result<Self, Self::Error> {
        Ok([
            v.txn_id.into(),
            v.txo.into(),
            sqlite::Value::try_from(v.slot_no)?,
        ])
    }
}

/// Update `spent_slot` in `txo_by_stake` table params.
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

impl TryFrom<UpdateTxoSpentParams> for [sqlite::Value; 5] {
    type Error = anyhow::Error;

    fn try_from(p: UpdateTxoSpentParams) -> Result<Self, Self::Error> {
        Ok([
            sqlite::Value::try_from(p.spent_slot)?,
            p.txn_index.into(),
            p.txo.into(),
            sqlite::Value::try_from(p.slot_no)?,
            sqlite::Value::try_from(p.spent_slot)?,
        ])
    }
}
