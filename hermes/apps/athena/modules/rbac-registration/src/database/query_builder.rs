//! SQLite query builders

pub(crate) struct QueryBuilder;

impl QueryBuilder {
    /// Select a root registration from a catalyst ID.
    /// The earliest (lowest `slot_no`, then lowest `txn_idx`) registration
    /// is considered the canonical/valid registration if multiple exist.
    pub(crate) fn select_root_reg_by_cat_id(table: &str) -> String {
        format!(
            r#"
            SELECT txn_id, slot_no, txn_idx
            FROM {table}
                WHERE prv_txn_id IS NULL
                AND problem_report IS NULL
                AND catalyst_id = ?
                ORDER BY slot_no ASC, txn_idx ASC
                LIMIT 1;
            "#
        )
    }

    /// Select a child registration from a parent registration
    /// The earliest (lowest `slot_no`, then lowest `txn_idx`) registration
    /// is considered the canonical/valid registration if multiple exist.
    ///
    /// The child is linked to the parent by the `prv_txn_id` field.
    pub(crate) fn select_child_reg_from_parent(table: &str) -> String {
        format!(
            r#"
            SELECT txn_id, slot_no, txn_idx
            FROM {table}
                WHERE prv_txn_id = ?
                AND problem_report IS NULL
                ORDER BY slot_no ASC, txn_idx ASC
                LIMIT 1;
            "#
        )
    }

    /// Select transaction ID that the stake address belongs to.
    /// Stake address can be replaced by the newest valid registration chain,
    /// so order by latest `slot` and latest `txn_idx`.
    ///
    /// The data from `rbac_stake_address` itself cannot indicate that the stake address
    /// is in a valid chain or not. Hence all associated data need to be checked.
    pub(crate) fn select_txn_id_by_stake_addr(table: &str) -> String {
        format!(
            r#"
            SELECT txn_id, slot_no, txn_idx
            FROM {table}
                WHERE stake_address = ?
                ORDER BY slot_no DESC, txn_idx DESC
            "#
        )
    }

    /// Get the registration information from the transaction ID.
    /// The earliest (lowest `slot_no`, then lowest `txn_idx`) registration
    /// is considered the canonical/valid registration if multiple exist.
    pub(crate) fn select_reg_by_txn_id(table: &str) -> String {
        format!(
            r#"
            SELECT prv_txn_id, slot_no, catalyst_id, txn_idx
            FROM {table}
                WHERE txn_id = ? AND problem_report IS NULL
                ORDER BY slot_no ASC, txn_idx ASC
            "#
        )
    }

    /// Find all root registration that has this given catalyst ID, lets say `catalyst_id_1`.
    /// It should come before the given slot_no - `slot_no_1` , and txn_idx - `txn_idx_1`.
    /// eg. The input is slot_no = 20, txn_idx = 2.
    /// It will give these registrations:
    /// slot_no = 20, txn_id = 1
    /// slot_no = 9 txn_id = 2
    /// slot_no = txn_id = 0
    /// This is used to validate that the given root is valid or not.
    /// Note that the valid root should have the least `slot_no` and least `txn_idx` with no problem report.
    ///
    /// If valid chain is found, the given catalyst ID (`catalyst_id_1`) with slot_no (`slot_no_1`) and
    /// txn_idx (`txn_idx_1`) is invalid.
    pub(crate) fn select_root_reg_by_cat_id_less_than_slot_txn_idx(table: &str) -> String {
        format!(
            r#"
            SELECT txn_id
            FROM {table} 
                WHERE prv_txn_id IS NULL
                AND problem_report IS NULL
                AND catalyst_id = ?
                AND (slot_no < ? OR (slot_no = ? AND txn_idx < ?))
            "#
        )
    }
}
