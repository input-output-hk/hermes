//! `UPDATE` queries.

use crate::{
    database::{sql, staked_ada::UpdateTxoSpentParams},
    utils::sqlite,
};

/// Sequentially updates by [`UpdateTxoSpentParams`].
pub fn update_txo_spent(
    conn: &mut sqlite::Connection,
    params: impl IntoIterator<Item = UpdateTxoSpentParams>,
) -> Result<usize, (usize, anyhow::Error)> {
    conn.prepare(sql::STAKED_ADA.update_txo_spent)
        .map_err(|err| (0, err))?
        .execute_iter(params)
}
