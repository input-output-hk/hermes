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
            import hermes:logging/api;
            import hermes:http-gateway/api;

            export hermes:init/event;
            export hermes:doc-sync/event;
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

impl exports::hermes::init::event::Guest for Component {
    /// Initialize the module.
    fn init() -> bool {
        log::init(log::LevelFilter::Trace);
        info!(target: "doc_sync::init", "Doc sync module initialized");
        true
    }
}

/// Stub event handler for receiving documents (not used in this publishing-only demo).
impl exports::hermes::doc_sync::event::Guest for Component {
    fn on_new_doc(
        _channel: ChannelName,
        _doc: DocData,
    ) {
        // Not implemented - this demo only shows publishing
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
            _ => Some(json_response(404, &serde_json::json!({"error": "Not found"}))),
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

/// Default channel name for doc-sync operations
const DOC_SYNC_CHANNEL: &str = "documents";

/// API for posting documents to IPFS `PubSub` channels.
pub mod channel {
    use super::{hermes, DocData, SyncChannel, DOC_SYNC_CHANNEL};

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
