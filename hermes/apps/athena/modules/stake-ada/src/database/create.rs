//! Create the database tables for RBAC registration.

use shared::{bindings::hermes::sqlite::api::Sqlite, utils::log::log_error};

/// Index of stake registrations.
/// Can also be used to convert a known stake key hash back to a full stake address.
const STAKE_REGISTRATION_TABLE: &str = r#"
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
    );
"#;

/// This could be ADA or a native asset being spent.
/// This can represent a spend on either immutable data or volatile data.
const TXI_BY_TXN_ID_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS txi_by_txn_id (
        txn_id          BLOB NOT NULL,          -- 32 Bytes Transaction Hash that was spent.
        txo             INTEGER NOT NULL,       -- Index of the TXO which was spent

        -- Non key data, we can only spend a transaction hash/txo once, so this should be unique in any event.
        slot_no         INTEGER NOT NULL,       -- slot number when the spend occurred.

        PRIMARY KEY (txn_id, txo)
    );
"#;

/// Transaction Outputs (ADA) per stake address.
/// Unstaked ADA is not present in this table.
const TXO_BY_STAKE_ADDRESS_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS txo_by_stake (
        -- Primary Key Fields
        stake_address   BLOB NOT NULL,          -- 29 Byte stake hash (CIP19).
        slot_no         INTEGER NOT NULL,       -- slot number the txo was created in.
        txn_index       INTEGER NOT NULL,       -- Which Transaction in the Slot is the TXO.
        txo             INTEGER NOT NULL,       -- offset in the txo list of the transaction the txo is in.

        -- Transaction Output Data
        address         BLOB NOT NULL,          -- TXO address (CIP19 Formatted Text).
        value           INTEGER NOT NULL,       -- Lovelace value of the TXO (u64).
        
        -- Data needed to correlate a spent TXO.
        txn_id          BLOB NOT NULL,          -- 32 byte hash of this transaction.

        spent_slot      INTEGER NOT NULL,       -- Slot this TXO was spent in.
                                                -- This is ONLY calculated/stored 
                                                -- when first detected in a query lookup.
                                                -- It serves as an optimization on subsequent queries. 

        PRIMARY KEY (stake_address, slot_no, txn_index, txo)
    );
"#;

/// Create a `known_stake_address` and `stake_registration` table.
pub(crate) fn create_stake_tables(sqlite: &Sqlite) {
    const FUNCTION_NAME: &str = "create_stake_tables";
    if let Err(e) = sqlite.execute(STAKE_REGISTRATION_TABLE) {
        log_error(
            file!(),
            FUNCTION_NAME,
            "hermes::sqlite::api::execute",
            &format!("Failed to create stake_registration table: {e}"),
            None,
        );
    }
    if let Err(e) = sqlite.execute(TXI_BY_TXN_ID_TABLE) {
        log_error(
            file!(),
            FUNCTION_NAME,
            "hermes::sqlite::api::execute",
            &format!("Failed to create txi_by_txn_id table: {e}"),
            None,
        );
    }
    if let Err(e) = sqlite.execute(TXO_BY_STAKE_ADDRESS_TABLE) {
        log_error(
            file!(),
            FUNCTION_NAME,
            "hermes::sqlite::api::execute",
            &format!("Failed to create txo_by_stake_address_table table: {e}"),
            None,
        );
    }
}
