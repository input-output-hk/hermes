//! Create the database tables for RBAC registration.

use shared::utils::sqlite;

/// Sequentially creates all tables if they don't exist in a transaction.
pub fn create_tables(conn: &mut sqlite::Connection) -> anyhow::Result<()> {
    let tx = conn.begin()?;

    // Index of stake registrations.
    // Can also be used to convert a known stake key hash back to a full stake address.
    tx.execute(r#"
        CREATE TABLE IF NOT EXISTS stake_registration (
            -- Primary Key Data
            stake_address    BLOB NOT NULL,         -- 29 Byte stake hash (CIP19).
            slot_no          INTEGER NOT NULL,      -- slot number when the key_was_registered/re-registered.
            txn_idx          INTEGER NOT NULL,      -- Index of the TX which holds the registration data.
        
            -- Non-Key Data
            stake_public_key BLOB,                  -- 32 Bytes Stake address - not present for scripts and may not be present for `register`.

            -- Stake key lifecycle data, shows what happened with the stake key at this slot#.
            script           BOOLEAN  NOT NULL,     -- Is the address a script address.
            register         BOOLEAN  NOT NULL,     -- True if the this transaction contains cardano stake registration cert.
            deregister       BOOLEAN  NOT NULL,     -- True if the this transaction contains cardano stake deregistration cert.
            cip36            BOOLEAN  NOT NULL,     -- True if the this transaction contains CIP36 registration.
            pool_delegation  BLOB NOT NULL,         -- Stake was delegated to this Pool address.
                                                    -- Not present if delegation did not change.

            PRIMARY KEY (stake_address, script, slot_no, txn_idx)
        )
    "#)?;

    // This could be ADA or a native asset being spent.
    // This can represent a spend on either immutable data or volatile data.
    tx.execute(r#"
        CREATE TABLE IF NOT EXISTS txi_by_txn_id (
            txn_id          BLOB NOT NULL,          -- 32 Bytes Transaction Hash that was spent.
            txo             INTEGER NOT NULL,       -- Index of the TXO which was spent

            -- Non key data, we can only spend a transaction hash/txo once, so this should be unique in any event.
            slot_no         INTEGER NOT NULL,       -- slot number when the spend occurred.

            PRIMARY KEY (txn_id, txo)
        );
    "#)?;

    // Transaction Outputs (ADA) per stake address.
    // ADA that isn't staked is not present in this table.
    tx.execute(r#"
        CREATE TABLE IF NOT EXISTS txo_by_stake_address (
            txn_id          BLOB NOT NULL,          -- 32 Bytes Transaction Hash that was spent.
            txo             INTEGER NOT NULL,       -- Index of the TXO which was spent

            -- Non key data, we can only spend a transaction hash/txo once, so this should be unique in any event.
            slot_no         INTEGER NOT NULL,       -- slot number when the spend occurred.

            PRIMARY KEY (txn_id, txo)
        );
    "#)?;

    tx.commit()?;
    Ok(())
}
