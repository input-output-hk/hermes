//! Create the database tables for RBAC registration.

use crate::{
    hermes::sqlite::{self, api::Sqlite},
    utils::log::log_error,
};

/// RBAC registration database schema.
const RBAC_REGISTRATION_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS rbac_registration (
        txn_id          BLOB NOT NULL,      -- 32 bytes of transaction ID (aka transaction hash)
        slot_no         INTEGER NOT NULL,   -- Slot number
        txn_idx         INTEGER NOT NULL,   -- Index of the transaction in the block
        prv_txn_id      BLOB,               -- 32 bytes of previous transaction ID (aka transaction hash)
        purpose         TEXT,               -- Registration purpose
        catalyst_id     TEXT,               -- Catalyst short ID - Exist only for Role0
        problem_report  TEXT,               -- Problem report

        PRIMARY KEY (txn_id)

    );
    -- Use for root lookup by catalyst_id
    CREATE INDEX IF NOT EXISTS idx_rbac_reg_cat_id ON rbac_registration (catalyst_id, slot_no, txn_idx);    
    -- Child lookup
    CREATE INDEX IF NOT EXISTS idx_rbac_reg_prv_tx ON rbac_registration (prv_txn_id, slot_no, txn_idx);
"#;

/// RBAC registration stake address database schema.
const RBAC_STAKE_ADDRESS_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS rbac_stake_address (
        stake_address   BLOB NOT NULL,      -- 29 bytes of stake hash (CIP19)
        slot_no         INTEGER NOT NULL,   -- Slot number
        txn_idx         INTEGER NOT NULL,   -- Index of the transaction in the block
        catalyst_id     TEXT,               -- Catalyst short ID - Exist only for Role0
        txn_id          BLOB NOT NULL,      -- 32 bytes of transaction ID (aka transaction hash)

        PRIMARY KEY (stake_address, txn_id)
    );
    -- Stake lookup (always want the newest registration first)
    CREATE INDEX IF NOT EXISTS idx_stake_addr ON rbac_stake_address (stake_address, slot_no DESC, txn_idx DESC);
"#;

/// Create a `rbac_registration` and `rbac_stake_address` table.
pub(crate) fn create_rbac_tables(sqlite: &Sqlite) {
    const FUNCTION_NAME: &str = "create_rbac_tables";
    if let Err(e) = sqlite.execute(RBAC_REGISTRATION_TABLE) {
        log_error(
            file!(),
            FUNCTION_NAME,
            "hermes::sqlite::api::execute",
            &format!("Failed to create rbac_registration table: {e}"),
            None,
        );
    }
    if let Err(e) = sqlite.execute(RBAC_STAKE_ADDRESS_TABLE) {
        log_error(
            file!(),
            FUNCTION_NAME,
            "hermes::sqlite::api::execute",
            &format!("Failed to create rbac_stake_address table: {e}"),
            None,
        );
    }
}
