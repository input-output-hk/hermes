//! Doc sync database portion.

mod delete;
mod insert;
mod select;
mod types;

pub use self::{delete::*, insert::*, select::*, types::*};
use crate::{database::sql, utils::sqlite};

/// Sequentially creates all tables if they don't exist.
///
/// # Errors
///
/// Returns an error if sqlite returns it during the execution or transaction operations
/// failed.
pub fn create_tables(conn: &mut sqlite::Connection) -> anyhow::Result<()> {
    conn.execute(sql::SCHEMA.doc_sync)?;
    Ok(())
}
