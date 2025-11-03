//! `DELETE` queries.

use crate::{database::sql, utils::sqlite};

/// Deletes entries since the slot number.
///
/// # Errors
///
/// Returns an error if sqlite returns it during the execution or query preparation.
pub fn delete_txo_assets_since_slot(
    conn: &mut sqlite::Connection,
    inclusive_slot_no: u64,
) -> anyhow::Result<()> {
    conn.prepare(sql::STAKED_ADA.delete_txo_assets_since_slot)?
        .execute(&[&inclusive_slot_no.try_into()?])
}

/// Deletes entries since the slot number.
///
/// # Errors
///
/// Returns an error if sqlite returns it during the execution or query preparation.
pub fn delete_stake_registration_since_slot(
    conn: &mut sqlite::Connection,
    inclusive_slot_no: u64,
) -> anyhow::Result<()> {
    conn.prepare(sql::STAKED_ADA.delete_stake_registration_since_slot)?
        .execute(&[&inclusive_slot_no.try_into()?])
}

/// Deletes entries since the slot number.
///
/// # Errors
///
/// Returns an error if sqlite returns it during the execution or query preparation.
pub fn delete_txi_since_slot(
    conn: &mut sqlite::Connection,
    inclusive_slot_no: u64,
) -> anyhow::Result<()> {
    conn.prepare(sql::STAKED_ADA.delete_txi_since_slot)?
        .execute(&[&inclusive_slot_no.try_into()?])
}

/// Deletes entries since the slot number.
///
/// # Errors
///
/// Returns an error if sqlite returns it during the execution or query preparation.
pub fn delete_txo_since_slot(
    conn: &mut sqlite::Connection,
    inclusive_slot_no: u64,
) -> anyhow::Result<()> {
    conn.prepare(sql::STAKED_ADA.delete_txo_since_slot)?
        .execute(&[&inclusive_slot_no.try_into()?])
}
