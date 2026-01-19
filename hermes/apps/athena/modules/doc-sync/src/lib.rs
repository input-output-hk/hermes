#![allow(missing_docs)]
//! Doc Sync - IPFS `PubSub` document publishing. See README.md for details.

shared::bindings_generate!({
    world: "hermes:app/hermes",
    path: "../../../../../wasm/wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            include wasi:cli/imports@0.2.6;
            import hermes:doc-sync/api;
            import hermes:ipfs/api;
            import hermes:logging/api;
            import hermes:http-gateway/api;

            export hermes:init/event;
            export hermes:ipfs/event;        // Required: Receives `PubSub` messages via on-topic
            export hermes:doc-sync/event-on-new-doc;    // Optional: Doc-sync specific events
            export hermes:doc-sync/event-document-provider;    // Optional: Doc-sync specific events
            export hermes:http-gateway/event;
        }
    ",
    share: ["hermes:logging"],
});

export!(Component);

use cid::{Cid, multihash::Multihash};
use hermes::{
    doc_sync::api::{DocData, SyncChannel},
    http_gateway::api::{Bstr, Headers, HttpGatewayResponse, HttpResponse},
};
use sha2::{Digest as _, Sha256};
use shared::{
    bindings::hermes::doc_sync::api::ChannelName,
    database::doc_sync::InsertDocumentRow,
    utils::{
        log::{self, error, info},
        sqlite,
    },
};

use crate::exports::hermes::doc_sync::event_document_provider::IpfsCid;

/// Doc Sync component - thin wrapper calling host-side implementation.
struct Component;

/// Default channel name for doc-sync operations
const DOC_SYNC_CHANNEL: &str = "documents";

/// Maximum length for message previews in log messages
const MESSAGE_PREVIEW_MAX_LEN: usize = 100;

/// Format a message for logging with size information and truncation.
///
/// Messages longer than `MESSAGE_PREVIEW_MAX_LEN` are truncated with "..." suffix.
fn format_message_preview(data: &[u8]) -> String {
    let preview = String::from_utf8_lossy(data);
    let size = data.len();
    if preview.len() > MESSAGE_PREVIEW_MAX_LEN {
        let truncated: String = preview.chars().take(MESSAGE_PREVIEW_MAX_LEN).collect();
        format!("{truncated}... ({size} bytes)")
    } else {
        format!("{preview} ({size} bytes)")
    }
}

/// Initialize the doc-sync database schema.
///
/// Creates the `document` table for either the persistent or in-memory database
/// depending on the `in_memory` flag.
///
/// # Errors
///
/// Returns any `SQLite` error that occurs while opening the connection,
/// beginning the transaction, creating tables, or committing the transaction.
fn init_db(in_memory: bool) -> anyhow::Result<()> {
    let mut conn = sqlite::Connection::open(in_memory)?;
    let mut tx = conn.begin()?;
    shared::database::doc_sync::create_tables(&mut tx)?;
    tx.commit()
}

impl exports::hermes::init::event::Guest for Component {
    /// Initialize the module.
    fn init() -> bool {
        log::init(log::LevelFilter::Trace);
        info!(target: "doc_sync::init", "Doc sync module initialized");
        if let Err(err) = init_db(false) {
            error!(target: "doc_sync::init", "Failed to initialize database: {err:?}");
            return false;
        }

        // Create the channel during initialization
        SyncChannel::new(DOC_SYNC_CHANNEL);
        true
    }
}

/// Event handler for receiving `PubSub` messages from IPFS layer.
///
/// This handler is REQUIRED for modules to receive `PubSub` messages. When a message
/// arrives via Gossipsub, the Hermes runtime dispatches it to this `on-topic` handler.
/// Without this export, the module cannot receive any `PubSub` messages.
impl exports::hermes::ipfs::event::Guest for Component {
    fn on_topic(message: hermes::ipfs::api::PubsubMessage) -> bool {
        log::init(log::LevelFilter::Trace);

        info!(
            target: "doc_sync::receiver",
            "📨 RECEIVED PubSub message on topic '{}': {}",
            message.topic,
            format_message_preview(&message.message)
        );

        true // Return true to indicate message was handled
    }
}

/// Event handler for doc-sync specific events (not currently used).
///
/// This is for potential future doc-sync specific event types. Currently, all
/// `PubSub` messages are received via the `on-topic` handler above.
impl exports::hermes::doc_sync::event_on_new_doc::Guest for Component {
    fn on_new_doc(
        channel: ChannelName,
        doc: DocData,
    ) {
        log::init(log::LevelFilter::Trace);

        info!(
            target: "doc_sync::receiver",
            "📨 RECEIVED PubSub message on channel '{}': {}",
            channel,
            format_message_preview(&doc)
        );

        if let Err(err) = store_in_db(&doc, &channel) {
            error!(target: "doc_sync::on_new_doc", "Failed to store doc from channel {channel}: {err:?}");
        }
    }
}

impl exports::hermes::doc_sync::event_document_provider::Guest for Component {
    fn return_cids(channel: ChannelName) -> Vec<IpfsCid> {
        get_documents_cids(&channel).unwrap_or_default()
    }

    fn retrieve_doc(
        _channel: ChannelName,
        cid: IpfsCid,
    ) -> std::option::Option<DocData> {
        get_document_by_cid(&cid).ok().flatten()
    }
}

/// Helper function to get documents cids by topic.
fn get_documents_cids(topic: &str) -> anyhow::Result<Vec<IpfsCid>> {
    let mut conn = sqlite::Connection::open(false)?;
    let docs = shared::database::doc_sync::get_documents_cids_by_topic(&mut conn, topic)?;
    Ok(docs.into_iter().map(|cid| cid.0.to_bytes()).collect())
}

/// Helper function to retrieve document by it's cid.
fn get_document_by_cid(cid: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
    let mut conn = sqlite::Connection::open(false)?;
    let document_data = shared::database::doc_sync::get_document_by_cid(&mut conn, cid)?;
    Ok(document_data.map(|data| data.cid))
}

/// HTTP Gateway endpoint for testing with curl.
///
/// POST /api/doc-sync/post - Post a document to the "documents" channel
///
/// Example:
/// ```bash
/// curl -X POST http://localhost:5000/api/doc-sync/post \
///   -H "Host: athena.hermes.local" \
///   -H "Content-Type: text/plain" \
///   -d "Hello, IPFS!"
/// ```
impl exports::hermes::http_gateway::event::Guest for Component {
    fn reply(
        body: Vec<u8>,
        _headers: Headers,
        path: String,
        method: String,
    ) -> Option<HttpGatewayResponse> {
        log::init(log::LevelFilter::Trace);
        info!(target: "doc_sync", "HTTP {method} {path}");

        match (method.as_str(), path.as_str()) {
            ("POST", "/api/doc-sync/post") => {
                // Call channel::post (executes 4-step workflow on host)
                match channel::post(&body) {
                    Ok(cid_bytes) => match TryInto::<Cid>::try_into(cid_bytes) {
                        Ok(cid) => Some(json_response(
                            200,
                            &serde_json::json!({
                                "success": true,
                                "cid": cid.to_string()
                            }),
                        )),
                        Err(e) => {
                            error!(target: "doc_sync", "Failed to convert CID bytes to CID: {e:?}");
                            Some(json_response(
                                500,
                                &serde_json::json!({
                                    "success": false,
                                    "error": "Failed to convert CID bytes to CID"
                                }),
                            ))
                        },
                    },
                    Err(e) => {
                        error!(target: "doc_sync", "Failed to post document: {e:?}");
                        Some(json_response(
                            500,
                            &serde_json::json!({
                                "success": false,
                                "error": "Failed to post document"
                            }),
                        ))
                    },
                }
            },
            _ => Some(json_response(
                404,
                &serde_json::json!({"error": "Not found"}),
            )),
        }
    }
}

/// Helper to create JSON HTTP responses.
fn json_response(
    code: u16,
    body: &serde_json::Value,
) -> HttpGatewayResponse {
    HttpGatewayResponse::Http(HttpResponse {
        code,
        headers: vec![(
            "content-type".to_string(),
            vec!["application/json".to_string()],
        )],
        body: Bstr::from(body.to_string()),
    })
}

/// API for posting documents to IPFS `PubSub` channels.
pub mod channel {
    use shared::utils::log::error;

    use super::{DOC_SYNC_CHANNEL, DocData, SyncChannel, hermes};
    use crate::store_in_db;

    /// Post a document to the "documents" channel. Returns the document's CID.
    ///
    /// # Errors
    ///
    /// Returns an error if the document cannot be posted to the channel.
    pub fn post(document_bytes: &DocData) -> Result<Vec<u8>, hermes::doc_sync::api::Errno> {
        // Create channel via host
        let channel = SyncChannel::new(DOC_SYNC_CHANNEL);
        // Post document via host (executes 4-step workflow in host)
        match channel.post(document_bytes) {
            Ok(cid) => {
                // If successfully posted, store document in db
                if let Err(err) = store_in_db(&document_bytes, DOC_SYNC_CHANNEL) {
                    error!(target: "doc_sync::channel::post", "Failed to store doc in db: {err:?}");
                }
                Ok(cid)
            },
            Err(err) => {
                error!(target: "doc_sync::channel::post", "Failed to post doc: {err:?}");
                Err(err)
            },
        }
    }
}

/// Stores the document in local `SQLite`: computes CID, stamps current time, and inserts
/// into `document` table.
fn store_in_db(
    doc_cbor: &DocData,
    topic: &str,
) -> anyhow::Result<()> {
    let cid = compute_cid(doc_cbor)?;
    let now = chrono::Utc::now();
    let row = InsertDocumentRow {
        cid,
        document: doc_cbor.clone(),
        inserted_at: now,
        topic: topic.to_string(),
        metadata: None,
    };

    let mut conn = sqlite::Connection::open(false)?;
    shared::database::doc_sync::insert_document(&mut conn, [row]).map_err(|(_, err)| err)?;
    Ok(())
}

/// Computes a `CIDv1` (CBOR codec, sha2-256 multihash) for the document bytes,
fn compute_cid(doc_cbor: &DocData) -> anyhow::Result<String> {
    const CBOR_CODEC: u64 = 0x51;
    const SHA2_256_CODE: u64 = 0x12;

    // `doc` is already in CBOR
    let hash = Sha256::digest(doc_cbor);
    let digest = Multihash::wrap(SHA2_256_CODE, &hash)?;
    let cid = Cid::new_v1(CBOR_CODEC, digest);
    Ok(cid.to_string())
}
