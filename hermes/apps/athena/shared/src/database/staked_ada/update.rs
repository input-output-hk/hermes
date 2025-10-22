//! `UPDATE` queries.

use crate::{
    database::{sql, staked_ada::UpdateTxoSpentParams},
    utils::sqlite,
};

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
