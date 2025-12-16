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
            export hermes:ipfs/event;        // Required: Receives PubSub messages via on-topic
            export hermes:doc-sync/event;    // Optional: Doc-sync specific events
            export hermes:http-gateway/event;
        }
    ",
    share: ["hermes:logging"],
});

export!(Component);

use hermes::{
    doc_sync::api::{DocData, SyncChannel},
    http_gateway::api::{Bstr, Headers, HttpGatewayResponse, HttpResponse},
};
use shared::{
    bindings::hermes::doc_sync::api::ChannelName,
    utils::log::{self, error, info},
};

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
        format!(
            "{}... ({} bytes)",
            &preview[..MESSAGE_PREVIEW_MAX_LEN],
            size
        )
    } else {
        format!("{} ({} bytes)", preview, size)
    }
}

impl exports::hermes::init::event::Guest for Component {
    /// Initialize the module.
    fn init() -> bool {
        log::init(log::LevelFilter::Trace);
        info!(target: "doc_sync::init", "Doc sync module initialized");
        // Create the channel during initialization
        SyncChannel::new(DOC_SYNC_CHANNEL);
        true
    }
}

/// Event handler for receiving PubSub messages from IPFS layer.
///
/// This handler is REQUIRED for modules to receive PubSub messages. When a message
/// arrives via Gossipsub, the Hermes runtime dispatches it to this `on-topic` handler.
/// Without this export, the module cannot receive any PubSub messages.
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
/// PubSub messages are received via the `on-topic` handler above.
impl exports::hermes::doc_sync::event::Guest for Component {
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
    }
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
                    Ok(cid_bytes) => {
                        let cid = String::from_utf8_lossy(&cid_bytes);
                        Some(json_response(
                            200,
                            &serde_json::json!({
                                "success": true,
                                "cid": cid
                            }),
                        ))
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
            _ => {
                Some(json_response(
                    404,
                    &serde_json::json!({"error": "Not found"}),
                ))
            },
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
        headers: vec![("content-type".to_string(), vec![
            "application/json".to_string(),
        ])],
        body: Bstr::from(body.to_string()),
    })
}

/// API for posting documents to IPFS `PubSub` channels.
pub mod channel {
    use super::{DOC_SYNC_CHANNEL, DocData, SyncChannel, hermes};

    /// Post a document to the "documents" channel. Returns the document's CID.
    ///
    /// # Errors
    ///
    /// Returns an error if the document cannot be posted to the channel.
    pub fn post(document_bytes: &DocData) -> Result<Vec<u8>, hermes::doc_sync::api::Errno> {
        // Create channel via host
        let channel = SyncChannel::new(DOC_SYNC_CHANNEL);
        // Post document via host (executes 4-step workflow in host)
        channel.post(document_bytes)
    }
}
