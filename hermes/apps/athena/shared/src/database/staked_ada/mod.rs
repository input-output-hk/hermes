//! Staked ADA modules database portion.

mod delete;
mod insert;
mod select;
mod types;
mod update;

pub use self::{delete::*, insert::*, select::*, types::*, update::*};
use crate::{database::sql, utils::sqlite};

/// Sequentially creates all tables if they don't exist.
pub fn create_tables(conn: &mut sqlite::Connection) -> anyhow::Result<()> {
    conn.execute(sql::SCHEMA.stake_registration)?;
    conn.execute(sql::SCHEMA.txi_by_txn_id)?;
    conn.execute(sql::SCHEMA.txo_assets_by_stake)?;
    conn.execute(sql::SCHEMA.txo_by_stake)
}
