//! Create the database tables for RBAC registration.

use shared::utils::sqlite;

/// Sequentially creates all tables if they don't exist in a transaction.
pub fn create_tables(conn: &mut sqlite::Connection) -> anyhow::Result<()> {
    let tx = conn.begin()?;

    tx.execute(include_str!(
        "../../../shared/src/sql/schema/stake_registration.sql"
    ))?;
    tx.execute(include_str!(
        "../../../shared/src/sql/schema/txi_by_txn_id.sql"
    ))?;
    tx.execute(include_str!(
        "../../../shared/src/sql/schema/txo_by_stake.sql"
    ))?;
    tx.execute(include_str!(
        "../../../shared/src/sql/schema/txo_assets_by_stake.sql"
    ))?;

    tx.commit()?;
    Ok(())
}
