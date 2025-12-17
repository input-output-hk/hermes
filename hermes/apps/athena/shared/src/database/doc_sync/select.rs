//! `SELECT` queries.

use crate::{
    database::{
        doc_sync::{DocumentRow, DocumentRowTuple},
        sql,
    },
    utils::sqlite,
};

/// Selects a document by its CID.
///
/// # Errors
///
/// Returns an error if sqlite returns it during data fetching or query preparation
/// or if the row cannot be converted into [`DocumentRow`].
pub fn get_document_by_cid(
    conn: &mut sqlite::Connection,
    cid: &[u8],
) -> anyhow::Result<Option<DocumentRow>> {
    let cid = sqlite::Value::from(cid.to_vec());
    conn.prepare(sql::DOC_SYNC.select_document_by_cid)?
        .query(&[&cid])?
        .map_as::<DocumentRowTuple>()
        .map(|res| res.map(Into::into))
        .next()
        .transpose()
}
