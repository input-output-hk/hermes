//! Staked ADA modules database.

mod delete;
mod insert;
mod select;
mod types;
mod update;

use crate::{database::sql, utils::sqlite};

pub use self::{delete::*, insert::*, select::*, types::*, update::*};

/// Sequentially creates all tables if they don't exist.
pub fn create_tables(conn: &mut sqlite::Connection) -> anyhow::Result<()> {
    let tx = conn.begin()?;

    tx.execute(sql::SCHEMA.stake_registration)?;
    tx.execute(sql::SCHEMA.txi_by_txn_id)?;
    tx.execute(sql::SCHEMA.txo_assets_by_stake)?;
    tx.execute(sql::SCHEMA.txo_by_stake)?;

    tx.commit()
}
