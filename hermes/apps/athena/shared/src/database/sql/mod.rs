//! Shared Athena SQL.

/// Shortcut for including SQL in an organized way.
macro_rules! include_sql {
    (#[dir = $dir:literal] pub const $const:ident: _ = $struct:ident {$($rel_stem:ident),* $(,)?};) => {
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
    };
}

include_sql! {
    #[dir = "schema"]
    pub const SCHEMA: _ = Schema {
        stake_registration,
        txi_by_txn_id,
        txo_assets_by_stake,
        txo_by_stake,
    };
}

include_sql! {
    #[dir = "queries"]
    pub const QUERIES: _ = Queries {
        delete_stake_registration_since_slot,
        delete_txi_since_slot,
        delete_txo_assets_since_slot,
        delete_txo_since_slot,
        select_txi_by_txn_ids,
        select_txo_assets_by_stake_address,
        select_txo_by_stake_address,
        update_txo_spent,
    };
}
