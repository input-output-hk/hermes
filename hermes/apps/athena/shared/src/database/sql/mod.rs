//! Shared Athena SQL.

/// Shortcut for including SQL in an organized way.
macro_rules! include_sql {
    (
        $(
            #[dir = $dir:literal]
            pub const $const:ident: _ = $struct:ident {
                $($rel_stem:ident),* $(,)?
            };
        )*
    ) => {
        $(
            #[doc = concat!("Shared SQL included from shared \"sql/", $dir, "\" directory")]
            #[derive(Debug)]
            pub struct $struct {
                $(
                    #[doc = "```text"]
                    #[doc = include_str!(concat!($dir, '/', stringify!($rel_stem), ".sql"))]
                    #[doc = "```"]
                    pub $rel_stem: &'static str,
                )*
            }

            #[doc = concat!("See [`", stringify!($struct), "`] documentation")]
            pub const $const: $struct = $struct {
                $($rel_stem: include_str!(concat!($dir, '/', stringify!($rel_stem), ".sql")),)*
            };

            impl $struct {
                /// Iterates over `<raw-sql-content>` of each field.
                pub fn iter() -> impl Iterator<Item = &'static str> {
                    Self::iter_named().map(|(_, sql)| sql)
                }

                /// Iterates over (`<raw-sql-name>`, `<raw-sql-content>`).
                pub fn iter_named() -> impl Iterator<Item = (&'static str, &'static str)> {
                    [$((stringify!($rel_stem), $const.$rel_stem),)*].into_iter()
                }
            }
        )*
    };
}

include_sql! {
    // SQL included from sql/schema directory.
    #[dir = "schema"]
    pub const SCHEMA: _ = Schema {
        stake_registration,
        txi_by_txn_id,
        txo_assets_by_stake,
        txo_by_stake,
        doc_sync
    };

    // SQL queries included from sql/staked_ada directory.
    #[dir = "staked_ada"]
    pub const STAKED_ADA: _ = StakedAda {
        delete_stake_registration_before_slot,
        delete_stake_registration_since_slot,
        delete_txi_before_slot,
        delete_txi_since_slot,
        delete_txo_assets_before_slot,
        delete_txo_assets_since_slot,
        delete_txo_before_slot,
        delete_txo_since_slot,
        insert_stake_registration,
        insert_txi_by_txn_id,
        insert_txo_assets_by_stake,
        insert_txo_by_stake,
        select_last_indexed_slot_no,
        select_txi_by_txn_id,
        select_txo_assets_by_stake,
        select_txo_by_stake,
        update_txo_spent,
    };

    // SQL queries included from sql/doc_sync directory.
    #[dir = "doc_sync"]
    pub const DOC_SYNC: _ = DocSync {
        delete_document_by_cid,
        insert_document,
        select_document_by_cid,
        select_documents_by_topic,
        select_documents_cids_by_topic,
        select_topics,
    };
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use anyhow::Context;
    use rusqlite::Connection;

    use super::*;

    /// Prepares each query string as a `SQLite` statement.
    /// An iterator is expected to be produced by `include_sql!` macro
    fn validate_raw_sql(
        conn: &Connection,
        named_sql: impl IntoIterator<Item = (&'static str, &'static str)>,
    ) -> anyhow::Result<()> {
        named_sql
            .into_iter()
            .try_for_each(|(name, sql)| conn.prepare(sql).map(drop).context(name))
    }

    fn execute_schema_sql(conn: &Connection) -> anyhow::Result<()> {
        conn.execute_batch(&Schema::iter().collect::<String>())
            .context("executing schema sql")
    }

    #[test]
    fn validate_staked_ada_sql() -> anyhow::Result<()> {
        let conn = Connection::open_in_memory()?;
        execute_schema_sql(&conn)?;
        validate_raw_sql(&conn, StakedAda::iter_named())
    }

    #[test]
    fn validate_doc_sync_sql() -> anyhow::Result<()> {
        let conn = Connection::open_in_memory()?;
        execute_schema_sql(&conn)?;
        validate_raw_sql(&conn, DocSync::iter_named())
    }
}
