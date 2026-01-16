//! `SQLite` queries types.

use chrono::{DateTime, Utc};
use derive_more::From;

use crate::utils::sqlite;

/// Row used when inserting into the `document` table.
#[derive(From)]
pub struct InsertDocumentRow {
    /// CID calculated over document bytes.
    pub cid: String,
    /// Document CBOR-encoded bytes.
    pub document: Vec<u8>,
    /// Timestamp when the document was inserted.
    pub inserted_at: DateTime<Utc>,
    /// Document topic.
    pub topic: String,
    /// Optional CBOR-encoded metadata associated with the document.
    pub metadata: Option<Vec<u8>>,
}

impl TryFrom<InsertDocumentRow> for [sqlite::Value; 5] {
    type Error = anyhow::Error;

    fn try_from(row: InsertDocumentRow) -> Result<Self, Self::Error> {
        Ok([
            row.cid.into(),
            row.document.into(),
            sqlite::Value::try_from(row.inserted_at)?,
            row.topic.into(),
            row.metadata.into(),
        ])
    }
}

/// Row returned when selecting documents by CID.
pub struct DocumentRow {
    /// Document CBOR-encoded bytes.
    pub document: Vec<u8>,
    /// Document cid.
    pub cid: Vec<u8>,
    /// Timestamp when the document was inserted.
    pub inserted_at: DateTime<Utc>,
    /// IPFS pubsub topic.
    pub topic: String,
    /// Optional CBOR-encoded metadata associated with the document.
    pub metadata: Option<Vec<u8>>,
}

/// [`DocumentRow`] represented by a tuple.
pub(super) type DocumentRowTuple = (Vec<u8>, Vec<u8>, DateTime<Utc>, String, Option<Vec<u8>>);

impl From<DocumentRowTuple> for DocumentRow {
    fn from((document, cid, inserted_at, topic, metadata): DocumentRowTuple) -> Self {
        Self {
            document,
            cid,
            inserted_at,
            topic,
            metadata,
        }
    }
}
