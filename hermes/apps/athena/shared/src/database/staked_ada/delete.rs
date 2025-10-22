//! `DELETE` queries.

use crate::{database::sql, utils::sqlite};

/// Deletes entries since the slot number.
pub fn delete_txo_assets_since_slot(
    conn: &mut sqlite::Connection,
    inclusive_slot_no: u64,
) -> anyhow::Result<()> {
    conn.prepare(sql::QUERIES.delete_txo_assets_since_slot)?
        .execute(&[&inclusive_slot_no.try_into()?])
}

/// Deletes entries since the slot number.
pub fn delete_stake_registration_since_slot(
    conn: &mut sqlite::Connection,
    inclusive_slot_no: u64,
) -> anyhow::Result<()> {
    conn.prepare(sql::QUERIES.delete_stake_registration_since_slot)?
        .execute(&[&inclusive_slot_no.try_into()?])
}

/// Deletes entries since the slot number.
pub fn delete_txi_since_slot(
    conn: &mut sqlite::Connection,
    inclusive_slot_no: u64,
) -> anyhow::Result<()> {
    conn.prepare(sql::QUERIES.delete_txi_since_slot)?
        .execute(&[&inclusive_slot_no.try_into()?])
}

/// Deletes entries since the slot number.
pub fn delete_txo_since_slot(
    conn: &mut sqlite::Connection,
    inclusive_slot_no: u64,
) -> anyhow::Result<()> {
    conn.prepare(sql::QUERIES.delete_txo_since_slot)?
        .execute(&[&inclusive_slot_no.try_into()?])
}
