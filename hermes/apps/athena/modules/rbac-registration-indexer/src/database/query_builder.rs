//! SQLite query builders

pub(crate) struct QueryBuilder;

impl QueryBuilder {
    /// Build create RBAC registration table.
    pub(crate) fn create_rbac_registration_table(table: &str) -> String {
        format!(
            r#"
            CREATE TABLE IF NOT EXISTS {table} (
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
            CREATE INDEX IF NOT EXISTS idx_rbac_reg_cat_id ON {table} (catalyst_id, slot_no, txn_idx);    
            -- Child lookup
            CREATE INDEX IF NOT EXISTS idx_rbac_reg_prv_tx ON {table} (prv_txn_id, slot_no, txn_idx);
            "#
        )
    }

    /// Build create RBAC stake address table.
    pub(crate) fn create_rbac_stake_address_table(table: &str) -> String {
        format!(
            r#"
            CREATE TABLE IF NOT EXISTS {table} (
                stake_address   BLOB NOT NULL,      -- 29 bytes of stake hash (CIP19)
                slot_no         INTEGER NOT NULL,   -- Slot number
                txn_idx         INTEGER NOT NULL,   -- Index of the transaction in the block
                catalyst_id     TEXT,               -- Catalyst short ID - Exist only for Role0
                txn_id          BLOB NOT NULL,      -- 32 bytes of transaction ID (aka transaction hash)

                PRIMARY KEY (stake_address, txn_id)
            );
            -- Stake lookup (always want the newest registration first)
            CREATE INDEX IF NOT EXISTS idx_stake_addr ON {table} (stake_address, slot_no DESC, txn_idx DESC);
            "#
        )
    }

    /// Build insert query for RBAC registration table.
    pub(crate) fn insert_rbac_registration(table: &str) -> String {
        format!(
            r#"
            INSERT OR REPLACE INTO {table} (
                txn_id, slot_no, txn_idx, prv_txn_id, purpose, catalyst_id, problem_report
            )
            VALUES(?, ?, ?, ?, ?, ?, ?);
            "#
        )
    }

    /// Build insert query for RBAC stake address table.
    pub(crate) fn insert_rbac_stake_address(table: &str) -> String {
        format!(
            r#"
            INSERT OR REPLACE INTO {table} (
                stake_address, slot_no, txn_idx, catalyst_id, txn_id
            )
            VALUES(?, ?, ?, ?, ?);
            "#
        )
    }

    /// Build delete query for immutable roll forward - volatile table.
    pub(crate) fn delete_roll_forward(table: &str) -> String {
        format!(
            r#"
            DELETE FROM {table} WHERE slot_no <= ?;
            "#
        )
    }

    /// Build delete query for roll backward - volatile table.
    pub(crate) fn delete_roll_back(table: &str) -> String {
        format!(
            r#"
            DELETE FROM {table} WHERE slot_no > ?;
            "#
        )
    }
}
