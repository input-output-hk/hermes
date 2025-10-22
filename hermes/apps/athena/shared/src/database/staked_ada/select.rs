//! `SELECT` queries.

use cardano_blockchain_types::{hashes::TransactionId, pallas_addresses::StakeAddress};

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

/// Select each matching [`TxoByStakeRowTuple`].
pub fn get_txo_by_stake_address(
    conn: &mut sqlite::Connection,
    stake_address: StakeAddress,
) -> anyhow::Result<Vec<TxoByStakeRow>> {
    conn.prepare(sql::QUERIES.select_txo_by_stake_address)?
        .query(&[&stake_address.into()])?
        .map_as::<TxoByStakeRowTuple>()
        .map(|res| res.map(Into::into))
        .collect()
}

/// Select each matching [`TxoAssetsByStakeRow`].
pub fn get_txo_assets_by_stake_address(
    conn: &mut sqlite::Connection,
    stake_address: StakeAddress,
) -> anyhow::Result<Vec<TxoAssetsByStakeRow>> {
    conn.prepare(sql::QUERIES.select_txo_assets_by_stake_address)?
        .query(&[&stake_address.into()])?
        .map_as::<TxoAssetsByStakeRowTuple>()
        .map(|res| res.map(Into::into))
        .collect()
}

/// Select each matching [`TxiByTxnIdRow`].
pub fn get_txi_by_txn_ids(
    conn: &mut sqlite::Connection,
    txn_ids: impl IntoIterator<Item = TransactionId>,
) -> anyhow::Result<Vec<TxiByTxnIdRow>> {
    let mut stmt = conn.prepare(sql::QUERIES.select_txi_by_txn_ids)?;
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
