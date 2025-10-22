//! Staked ADA modules database.

use cardano_blockchain_types::{
    hashes::TransactionId,
    pallas_addresses::StakeAddress,
    pallas_primitives::{AssetName, BigInt, PolicyId},
};
use derive_more::From;
use crate::{database::sql, utils::sqlite};

/// Sequentially creates all tables if they don't exist.
pub fn create_tables(conn: &mut sqlite::Connection) -> anyhow::Result<()> {
    let tx = conn.begin()?;

    tx.execute(sql::SCHEMA.stake_registration)?;
    tx.execute(sql::SCHEMA.txi_by_txn_id)?;
    tx.execute(sql::SCHEMA.txo_assets_by_stake)?;
    tx.execute(sql::SCHEMA.txo_by_stake)?;

    tx.commit()
}

/// [`TxoByStakeAddressRow`] represented by a tuple.
type TxoByStakeAddressRowInner = (TransactionId, u16, u16, u64, BigInt, Option<u64>);

/// Get UTXO assets query data.
#[derive(From)]
pub struct TxoByStakeAddressRow {
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

/// Select each [`TxoByStakeAddressRow`].
pub fn get_txo_by_stake_address(
    conn: &mut sqlite::Connection,
    stake_address: StakeAddress,
) -> anyhow::Result<Vec<TxoByStakeAddressRowInner>> {
    conn.prepare(sql::QUERIES.get_txo_by_stake_address)?
        .query(&[&stake_address.into()])?
        .map_as::<TxoByStakeAddressRowInner>()
        .map(|res| res.map(TxoByStakeAddressRowInner::from))
        .collect()
}

/// [`AssetsByStakeAddressRow`] represented by a tuple.
type AssetsByStakeAddressRowInner = (u16, u16, u64, PolicyId, AssetName, BigInt);

/// UTXO assets query row.
#[derive(From)]
pub struct AssetsByStakeAddressRow {
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

/// Select each [`AssetsByStakeAddressRow`].
pub fn get_assets_by_stake_address(
    conn: &mut sqlite::Connection,
    stake_address: StakeAddress,
) -> anyhow::Result<Vec<AssetsByStakeAddressRow>> {
    conn.prepare(sql::QUERIES.get_assets_by_stake_address)?
        .query(&[&stake_address.into()])?
        .map_as::<AssetsByStakeAddressRowInner>()
        .map(|res| res.map(AssetsByStakeAddressRow::from))
        .collect()
}

/// [`TxiByTxnIdsRow`] represented by a tuple.
type TxiByTxnIdsRowInner = (TransactionId, u16, u64);

/// TXI query data.
#[derive(From)]
pub struct TxiByTxnIdsRow {
    /// TXI transaction hash.
    pub txn_id: TransactionId,
    /// TXI original TXO index.
    pub txo: u16,
    /// TXI slot number.
    pub slot_no: u64,
}

/// Select each [`TxiByTxnIdsRow`].
pub fn get_txi_by_txn_ids(
    conn: &mut sqlite::Connection,
    txn_ids: impl IntoIterator<Item = TransactionId>,
) -> anyhow::Result<Vec<TxiByTxnIdsRow>> {
    let mut stmt = conn.prepare(sql::QUERIES.get_txi_by_txn_ids)?;
    txn_ids
        .into_iter()
        .map(|txn_id| {
            stmt.query(&[&txn_id.into()])?
                .map_as::<TxiByTxnIdsRowInner>()
                .map(|res| res.map(TxiByTxnIdsRow::from))
                .collect::<Result<Vec<_>, _>>()
        })
        .try_fold(vec![], |mut rows, res| {
            res.map(|mut next_rows| {
                rows.append(&mut next_rows);
                rows
            })
        })
}

/// Deletes entries since the slot number.
pub fn delete_assets_since_slot(
    conn: &mut sqlite::Connection,
    inclusive_slot_no: u64,
) -> anyhow::Result<()> {
    conn.prepare(sql::QUERIES.delete_assets_since_slot)?
        .execute(&[&inclusive_slot_no.try_into()?])
}

/// Deletes entries since the slot number.
pub fn delete_stake_registration_since_slot(
    conn: &mut sqlite::Connection,
    inclusive_slot_no: u64,
) -> anyhow::Result<()> {
    conn.prepare(sql::QUERIES.delete_stake_registration_since_slot)?
        .execute(&[&inclusive_slot_no.try_into()?])
}

/// Deletes entries since the slot number.
pub fn delete_txi_since_slot(
    conn: &mut sqlite::Connection,
    inclusive_slot_no: u64,
) -> anyhow::Result<()> {
    conn.prepare(sql::QUERIES.delete_txi_since_slot)?
        .execute(&[&inclusive_slot_no.try_into()?])
}

/// Deletes entries since the slot number.
pub fn delete_txo_since_slot(
    conn: &mut sqlite::Connection,
    inclusive_slot_no: u64,
) -> anyhow::Result<()> {
    conn.prepare(sql::QUERIES.delete_txo_since_slot)?
        .execute(&[&inclusive_slot_no.try_into()?])
}

/// TXO spent query params.
#[derive(From)]
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

/// Updates by each [`UpdateTxoSpentParams`].
/// When one of the updates fails, immediately returns.
/// Additionally, returns the number of successful updates.
pub fn update_txo_spent(
    conn: &mut sqlite::Connection,
    params: impl IntoIterator<Item = UpdateTxoSpentParams>,
) -> Result<usize, (usize, anyhow::Error)> {
    let mut stmt = conn
        .prepare(sql::QUERIES.update_txo_spent)
        .map_err(|err| (0, err))?;

    params
        .into_iter()
        .map(|p| {
            Ok([
                sqlite::Value::try_from(p.spent_slot)?,
                p.txn_index.into(),
                p.txo.into(),
                sqlite::Value::try_from(p.slot_no)?,
            ])
        })
        .map(|conversion_res| conversion_res.and_then(|p| stmt.execute(&p.each_ref())))
        .try_fold(0, |num_successful, res| {
            res.map(|()| num_successful + 1)
                .map_err(|err| (num_successful, err))
        })
}
