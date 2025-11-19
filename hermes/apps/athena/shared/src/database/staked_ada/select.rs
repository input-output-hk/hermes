//! `SELECT` queries.

use cardano_blockchain_types::{StakeAddress, hashes::TransactionId};

use crate::{
    database::{
        sql,
        staked_ada::{
            TxiByTxnIdRow, TxiByTxnIdsRowTuple, TxoAssetsByStakeRow, TxoAssetsByStakeRowTuple,
            TxoByStakeRow, TxoByStakeRowTuple,
        },
    },
    utils::sqlite,
};

/// Select the last scanned and indexed slot number.
///
/// # Errors
///
/// - `SQLite` access and deserialization
pub fn get_last_indexed_slot_no(conn: &mut sqlite::Connection) -> anyhow::Result<Option<u64>> {
    conn.prepare(sql::STAKED_ADA.select_last_indexed_slot_no)?
        .query_one(&[], |row| Ok(row.get(0).and_then(u64::try_from).ok()))
}

/// Select each matching [`TxoByStakeRow`].
///
/// # Errors
///
/// Returns an error if sqlite returns it during
/// data fetching, query preparation or
/// in case of sqlite type could not be represented as `TxoByStakeRowTuple`.
pub fn get_txo_by_stake_address(
    conn: &mut sqlite::Connection,
    stake_address: StakeAddress,
) -> anyhow::Result<Vec<TxoByStakeRow>> {
    conn.prepare(sql::STAKED_ADA.select_txo_by_stake)?
        .query(&[&stake_address.into()])?
        .map_as::<TxoByStakeRowTuple>()
        .map(|res| res.map(Into::into))
        .collect()
}

/// Select each matching [`TxoAssetsByStakeRow`].
///
/// # Errors
///
/// Returns an error if sqlite returns it during
/// data fetching, query preparation or
/// in case of sqlite type could not be represented as `TxoAssetsByStakeRowTuple`.
pub fn get_txo_assets_by_stake_address(
    conn: &mut sqlite::Connection,
    stake_address: StakeAddress,
) -> anyhow::Result<Vec<TxoAssetsByStakeRow>> {
    conn.prepare(sql::STAKED_ADA.select_txo_assets_by_stake)?
        .query(&[&stake_address.into()])?
        .map_as::<TxoAssetsByStakeRowTuple>()
        .map(|res| res.map(Into::into))
        .collect()
}

/// Select each matching [`TxiByTxnIdRow`].
///
/// # Errors
///
/// Returns an error if sqlite returns it during
/// data fetching, query preparation or
/// in case of sqlite type could not be represented as `TxiByTxnIdsRowTuple`.
pub fn get_txi_by_txn_ids(
    conn: &mut sqlite::Connection,
    txn_ids: impl IntoIterator<Item = TransactionId>,
) -> anyhow::Result<Vec<TxiByTxnIdRow>> {
    let mut stmt = conn.prepare(sql::STAKED_ADA.select_txi_by_txn_id)?;
    txn_ids
        .into_iter()
        .map(|txn_id| {
            stmt.query(&[&txn_id.into()])?
                .map_as::<TxiByTxnIdsRowTuple>()
                .map(|res| res.map(Into::into))
                .collect::<Result<Vec<_>, _>>()
        })
        .try_fold(vec![], |mut rows, res| {
            res.map(|mut next_rows| {
                rows.append(&mut next_rows);
                rows
            })
        })
}
