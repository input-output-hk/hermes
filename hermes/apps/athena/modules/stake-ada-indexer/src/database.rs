//! Create the database tables for RBAC registration.

use shared::{db, utils::sqlite};

/// Sequentially creates all tables if they don't exist in a transaction.
pub fn create_tables(conn: &mut sqlite::Connection) -> anyhow::Result<()> {
    let tx = conn.begin()?;

    tx.execute(db::SCHEMA.stake_registration)?;
    tx.execute(db::SCHEMA.txi_by_txn_id)?;
    tx.execute(db::SCHEMA.txo_assets_by_stake)?;
    tx.execute(db::SCHEMA.txo_by_stake)?;

    tx.commit()?;
    Ok(())
}
