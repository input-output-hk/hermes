//! `INSERT` queries.

use crate::{
    database::{doc_sync::InsertDocumentRow, sql},
    utils::sqlite,
};

/// Sequentially inserts [`InsertDocumentRow`] values into the `document` table.
///
/// # Errors
///
/// Returns an error if sqlite returns it during the execution or query preparation.
pub fn insert_document(
    conn: &mut sqlite::Connection,
    values: impl IntoIterator<Item = InsertDocumentRow>,
) -> Result<usize, (usize, anyhow::Error)> {
    conn.prepare(sql::DOC_SYNC.insert_document)
        .map_err(|err| (0, err))?
        .execute_iter(values)
}
