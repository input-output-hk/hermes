use crate::{
    hermes::hermes::{self, sqlite::{self, api::Sqlite}},
    utils::log::{log_error, log_info},
};

const FILE_NAME: &str = "rbac-registration/src/database/create.rs";

const RBAC_REGISTRATION_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS rbac_registration (
        txn_id          BLOB NOT NULL,      -- 32 bytes of transaction ID (aka transaction hash)
        slot_no         INTEGER NOT NULL,  -- Slot number
        txn_idx         INTEGER NOT NULL,  -- Index of the transaction in the block
        prv_txn_id      BLOB,     -- 32 bytes of previous transaction ID (aka transaction hash)
        purpose         TEXT,              -- Registration purpose
        catalyst_id     TEXT,              -- Catalyst short ID - Exist only for Role0
        problem_report  TEXT,              -- Problem report

        PRIMARY KEY (txn_id, slot_no)

    );
    CREATE INDEX IF NOT EXISTS prv_txn_id_index ON rbac_registration (prv_txn_id, slot_no);
    CREATE INDEX IF NOT EXISTS catalyst_id_index ON rbac_registration (catalyst_id);
"#;

const RBAC_STAKE_ADDRESS_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS rbac_stake_address (
        stake_address   BLOB NOT NULL,      -- 29 bytes of stake hash (CIP19)
        slot_no         INTEGER NOT NULL,   -- Slot number
        txn_idx         INTEGER NOT NULL,   -- Index of the transaction in the block
        catalyst_id     TEXT,               -- Catalyst short ID

        PRIMARY KEY (stake_address, slot_no, txn_idx)
    )
"#;

pub(crate) fn create_rbac_tables(sqlite: &Sqlite) {
    const FUNCTION_NAME: &str = "create_rbac_tables";
    if let Err(e) = sqlite.execute(RBAC_REGISTRATION_TABLE) {
        log_error(
            FILE_NAME,
            FUNCTION_NAME,
            "hermes::sqlite::api::execute",
            &format!("🚨 Failed to create rbac_registration table: {e}"),
            None,
        );
    }
    if let Err(e) = sqlite.execute(RBAC_STAKE_ADDRESS_TABLE) {
        log_error(
            FILE_NAME,
            FUNCTION_NAME,
            "hermes::sqlite::api::execute",
            &format!("🚨 Failed to create rbac_stake_address table: {e}"),
            None,
        );
    }
}
