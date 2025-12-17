//! `DELETE` queries.

use crate::{database::sql, utils::sqlite};

/// Deletes document by CID.
///
/// # Errors
///
/// Returns an error if sqlite returns it during the execution or query preparation.
pub fn delete_document_by_cid(
    conn: &mut sqlite::Connection,
    cid: &[u8],
) -> anyhow::Result<()> {
    conn.prepare(sql::DOC_SYNC.delete_document_by_cid)?
        .execute(&[&sqlite::Value::from(cid.to_vec())])
}
