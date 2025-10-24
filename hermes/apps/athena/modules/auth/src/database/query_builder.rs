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
}
