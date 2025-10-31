//! `INSERT` queries.

use crate::{
    database::{
        sql,
        staked_ada::{StakeRegistrationRow, TxiByTxnIdRow, TxoAssetsByStakeRow, TxoByStakeRow},
    },
    utils::sqlite,
};

/// Sequentially inserts [`StakeRegistrationRow`].
///
/// # Errors
///
/// Returns an error if sqlite returns it during the execution or query preparation.
pub fn insert_stake_registration(
    conn: &mut sqlite::Connection,
    values: impl IntoIterator<Item = StakeRegistrationRow>,
) -> Result<usize, (usize, anyhow::Error)> {
    conn.prepare(sql::STAKED_ADA.insert_stake_registration)
        .map_err(|err| (0, err))?
        .execute_iter(values)
}

/// Sequentially inserts [`StakeRegistrationRow`].
///
/// # Errors
///
/// Returns an error if sqlite returns it during the execution or query preparation.
pub fn insert_txi_by_txn_id(
    conn: &mut sqlite::Connection,
    values: impl IntoIterator<Item = TxiByTxnIdRow>,
) -> Result<usize, (usize, anyhow::Error)> {
    conn.prepare(sql::STAKED_ADA.insert_txi_by_txn_id)
        .map_err(|err| (0, err))?
        .execute_iter(values)
}

/// Sequentially inserts [`TxoAssetsByStakeRow`].
///
/// # Errors
///
/// Returns an error if sqlite returns it during the execution or query preparation.
pub fn insert_txo_assets_by_stake(
    conn: &mut sqlite::Connection,
    values: impl IntoIterator<Item = TxoAssetsByStakeRow>,
) -> Result<usize, (usize, anyhow::Error)> {
    conn.prepare(sql::STAKED_ADA.insert_txo_assets_by_stake)
        .map_err(|err| (0, err))?
        .execute_iter(values)
}

/// Sequentially inserts [`TxoByStakeRow`].
///
/// # Errors
///
/// Returns an error if sqlite returns it during the execution or query preparation.
pub fn insert_txo_by_stake(
    conn: &mut sqlite::Connection,
    values: impl IntoIterator<Item = TxoByStakeRow>,
) -> Result<usize, (usize, anyhow::Error)> {
    conn.prepare(sql::STAKED_ADA.insert_txo_by_stake)
        .map_err(|err| (0, err))?
        .execute_iter(values)
}
