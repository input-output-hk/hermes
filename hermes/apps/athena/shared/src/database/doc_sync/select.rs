//! `SELECT` queries.

use crate::{
    database::{
        doc_sync::{DocumentRow, DocumentRowTuple},
        sql,
    },
    utils::{common::types::Cid, sqlite},
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
    conn.prepare(sql::DOC_SYNC.select_document_by_cid)?
        .query(&[&cid.to_vec().into()])?
        .map_as::<DocumentRowTuple>()
        .map(|res| res.map(Into::into))
        .next()
        .transpose()
}

/// Selects documents by topic.
///
/// # Errors
///
/// Returns an error if sqlite returns it during data fetching or query preparation
/// or if the row cannot be converted into [`DocumentRow`].
pub fn get_documents_by_topic(
    conn: &mut sqlite::Connection,
    topic: &str,
) -> anyhow::Result<Vec<DocumentRow>> {
    conn.prepare(sql::DOC_SYNC.select_documents_by_topic)?
        .query(&[&topic.into()])?
        .map_as::<DocumentRowTuple>()
        .map(|res| res.map(Into::into))
        .collect::<anyhow::Result<Vec<DocumentRow>>>()
}

/// Selects documents cids by topic.
///
/// # Errors
///
/// Returns an error if sqlite returns it during data fetching or query preparation
/// or if the row cannot be converted into [`DocumentRow`].
pub fn get_documents_cids_by_topic(
    conn: &mut sqlite::Connection,
    topic: &str,
) -> anyhow::Result<Vec<Cid>> {
    conn.prepare(sql::DOC_SYNC.select_documents_cids_by_topic)?
        .query(&[&topic.into()])?
        .and_then(|row_result| {
            let row = row_result?;

            let bytes = row
                .get_as::<Vec<u8>>(0)
                .map_err(|err| anyhow::anyhow!("get cids by topic error: {err}"))?;

            let cid = Cid::try_from_bytes(&bytes)?;

            Ok(cid)
        })
        .collect()
}
