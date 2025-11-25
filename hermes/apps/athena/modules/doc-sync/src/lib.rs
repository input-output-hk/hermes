#![allow(missing_docs)]
//! # Doc Sync Module
//!
//! Demonstrates the mechanics of posting documents to IPFS and publishing via PubSub.
//!
//! ## 4-Step Publishing Workflow
//! 1. **Add to IPFS** (file_add) - Store document, get CID
//! 2. **Pin document** (file_pin) - Ensure persistence
//! 3. **Pre-publish** (TODO #630) - Validation/signing placeholder
//! 4. **Publish to PubSub** (pubsub_publish) - Broadcast to topic "doc-sync/{channel}"
//!
//! ## Usage
//! ```rust
//! let cid = channel::post(document_bytes)?;
//! ```

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
            export hermes:doc-sync/event;
            export hermes:doc-sync/api;
            export hermes:http-gateway/event;
        }
    ",
    share: ["hermes:logging", "hermes:doc-sync"],
});

export!(Component);

use shared::{
    bindings::hermes::doc_sync::api::{ChannelName, DocData},
    utils::log::{self, info},
};

use hermes::http_gateway::api::{Bstr, Headers, HttpGatewayResponse, HttpResponse};

use hermes::ipfs::api::{file_add, file_pin, pubsub_publish};

/// Doc Sync component implementing the IPFS PubSub demo.
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

/// Internal representation of the SyncChannel resource
pub struct SyncChannelImpl {
    name: ChannelName,
}

// Implement the doc-sync API functions
impl exports::hermes::doc_sync::api::Guest for Component {
    type SyncChannel = SyncChannelImpl;

    /// Get the CID for a document by adding it to IPFS (without pinning or publishing).
    fn id_for(doc: DocData) -> Vec<u8> {
        match file_add(&doc) {
            Ok(ipfs_path) => {
                let cid_str = ipfs_path.strip_prefix("/ipfs/").unwrap_or(&ipfs_path);
                cid_str.as_bytes().to_vec()
            }
            Err(_) => b"error".to_vec(),
        }
    }
}

// Implement the SyncChannel resource
impl exports::hermes::doc_sync::api::GuestSyncChannel for SyncChannelImpl {
    /// Create a new SyncChannel for publishing documents.
    fn new(name: ChannelName) -> SyncChannelImpl {
        SyncChannelImpl { name: name.clone() }
    }

    /// Close a SyncChannel (stub - no cleanup needed).
    fn close(
        &self,
        _name: ChannelName,
    ) -> Result<bool, exports::hermes::doc_sync::api::Errno> {
        Ok(true)
    }

    /// Post a document to IPFS and broadcast it via PubSub.
    ///
    /// Executes the 4-step workflow:
    /// 1. Add to IPFS (file_add) - store document, get CID
    /// 2. Pin (file_pin) - prevent garbage collection
    /// 3. Pre-publish (TODO #630) - validation/signing placeholder
    /// 4. Publish to PubSub (pubsub_publish) - broadcast to "doc-sync/{channel}"
    ///
    /// Returns the document's CID on success.
    fn post(
        &self,
        doc: DocData,
    ) -> Result<Vec<u8>, exports::hermes::doc_sync::api::Errno> {
        info!(target: "doc_sync", "📤 Posting {} bytes to channel: {}", doc.len(), self.name);

        // Step 1: Add document to IPFS (file_add)
        let ipfs_path = match file_add(&doc) {
            Ok(path) => {
                info!(target: "doc_sync", "✓ Step 1/4: Added to IPFS → {}", path);
                path
            }
            Err(e) => {
                info!(target: "doc_sync", "✗ Step 1/4 failed: file_add error: {:?}", e);
                return Err(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder);
            }
        };

        // Step 2: Pin the document (file_pin)
        match file_pin(&ipfs_path) {
            Ok(_) => info!(target: "doc_sync", "✓ Step 2/4: Pinned → {}", ipfs_path),
            Err(e) => {
                info!(target: "doc_sync", "✗ Step 2/4 failed: file_pin error: {:?}", e);
                return Err(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder);
            }
        }

        // Step 3: Pre-publish validation (TODO #630)
        info!(target: "doc_sync", "⏭ Step 3/4: Pre-publish (skipped - TODO #630)");

        // Step 4: Publish to PubSub (pubsub_publish)
        let topic = format!("doc-sync/{}", self.name);
        match pubsub_publish(&topic, &doc) {
            Ok(_) => info!(target: "doc_sync", "✓ Step 4/4: Published to PubSub → {}", topic),
            Err(e) => {
                info!(target: "doc_sync", "✗ Step 4/4 failed: pubsub_publish error: {:?}", e);
                return Err(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder);
            }
        }

        // Extract CID from path and return it
        let cid_str = ipfs_path.strip_prefix("/ipfs/").unwrap_or(&ipfs_path);
        Ok(cid_str.as_bytes().to_vec())
    }

    /// Stub - not implemented.
    fn prove_includes(
        &self,
        _loc: Vec<u8>,
        _provers: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>, exports::hermes::doc_sync::api::Errno> {
        Ok(vec![])
    }

    /// Stub - not implemented.
    fn prove_excludes(
        &self,
        _loc: Vec<u8>,
        _provers: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>, exports::hermes::doc_sync::api::Errno> {
        Ok(vec![])
    }

    /// Stub - not implemented.
    fn get(
        &self,
        _loc: Vec<u8>,
    ) -> Result<Vec<u8>, exports::hermes::doc_sync::api::Errno> {
        Err(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder)
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
        info!(target: "doc_sync", "HTTP {} {}", method, path);

        match (method.as_str(), path.as_str()) {
            ("POST", "/api/doc-sync/post") => {
                // Call channel::post which executes the full 4-step workflow
                match channel::post(body) {
                    Ok(cid_bytes) => {
                        let cid = String::from_utf8_lossy(&cid_bytes);
                        json_response(
                            200,
                            serde_json::json!({
                                "success": true,
                                "cid": cid.to_string()
                            }),
                        )
                    },
                    Err(_) => json_response(
                        500,
                        serde_json::json!({
                            "success": false,
                            "error": "Failed to post document"
                        }),
                    ),
                }
            },
            _ => json_response(404, serde_json::json!({"error": "Not found"})),
        }
    }
}

/// Helper to create JSON HTTP responses.
fn json_response(
    code: u16,
    body: serde_json::Value,
) -> Option<HttpGatewayResponse> {
    Some(HttpGatewayResponse::Http(HttpResponse {
        code,
        headers: vec![(
            "content-type".to_string(),
            vec!["application/json".to_string()],
        )],
        body: Bstr::from(body.to_string()),
    }))
}

/// Simple API for posting documents to IPFS PubSub.
///
/// Usage: `let cid = channel::post(document_bytes)?;`
pub mod channel {
    use super::*;
    use exports::hermes::doc_sync::api::GuestSyncChannel;

    /// Post a document to the "documents" channel. Returns the document's CID.
    pub fn post(document_bytes: DocData) -> Result<Vec<u8>, exports::hermes::doc_sync::api::Errno> {
        let channel = SyncChannelImpl {
            name: "documents".to_string(),
        };
        channel.post(document_bytes)
    }
}
