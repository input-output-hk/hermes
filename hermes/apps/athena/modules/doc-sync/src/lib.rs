#![allow(missing_docs)]
//! # Doc Sync Module
//!
//! Demonstrates IPFS PubSub-based document synchronization between apps.
//!
//! ## Overview
//! This module shows the mechanics of posting documents to IPFS and distributing
//! them via PubSub. The complete pub/sub flow requires **PR #691** to route
//! incoming messages to the `on_new_doc` event handler.
//!
//! ## 4-Step Publishing Workflow
//! 1. **Add to IPFS** - Store document, get CID
//! 2. **Pin document** - Ensure persistence
//! 3. **Pre-publish** - Validation (TODO #630)
//! 4. **Publish to PubSub** - Broadcast to subscribers
//!
//! ## PR #691 Integration (REQUIRED for subscriptions)
//! PR #691 adds the infrastructure to:
//! - Detect DocSync subscriptions (topics starting with "doc-sync/")
//! - Route PubSub messages to `doc_sync_topic_message_handler()`
//! - Validate messages using `CatalystSignedDocument`
//! - Trigger `on_new_doc` events on subscribed modules
//!
//! **Without PR #691:** Publishing works, but subscribers won't receive events.

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
    bindings::hermes::doc_sync::api::{ChannelName, DocData, SyncChannel},
    utils::log::{self, info},
};

use hermes::http_gateway::api::{Bstr, Headers, HttpGatewayResponse, HttpResponse};

use hermes::ipfs::api::{file_add, file_pin, pubsub_publish, pubsub_subscribe};

/// Doc Sync component implementing the IPFS PubSub demo.
struct Component;

impl exports::hermes::init::event::Guest for Component {
    /// Initialize the module and subscribe to the default "documents" channel.
    ///
    /// This is called once when the module loads. It subscribes to the PubSub topic
    /// "doc-sync/documents" so this module will receive documents posted by others.
    ///
    /// **REQUIRES PR #691:** Without it, subscription succeeds but `on_new_doc` won't be triggered.
    fn init() -> bool {
        log::init(log::LevelFilter::Trace);
        info!(target: "doc_sync::init", "Opening channel...");

        // Subscribe to receive documents on the "documents" channel
        // This calls pubsub_subscribe("doc-sync/documents")
        let _chan = SyncChannel::new("documents");

        info!(target: "doc_sync::init", "Channel opened");
        true
    }
}

/// Event handler called when a document arrives on a subscribed channel.
///
/// **Requires PR #691** to be triggered. The PR adds routing from PubSub messages to this handler.
/// Currently subscriptions work but this event won't fire until PR #691 merges.
///
/// Flow: `SyncChannel::new("documents")` subscribes → another app calls `channel::post(doc)` →
/// PR #691 routes the PubSub message → this handler is called with the document.
impl exports::hermes::doc_sync::event::Guest for Component {
    fn on_new_doc(
        channel: ChannelName,
        doc: DocData,
    ) {
        log::init(log::LevelFilter::Trace);
        info!(target: "doc_sync", "📥 Received doc on channel '{}': {} bytes", channel, doc.len());

        // TODO: Process document (validate, store, trigger workflows)
    }
}

/// Internal representation of the SyncChannel resource
pub struct SyncChannelImpl {
    name: ChannelName,
}

// Implement the doc-sync API functions
impl exports::hermes::doc_sync::api::Guest for Component {
    type SyncChannel = SyncChannelImpl;

    /// Get the Content ID (CID) for a document by adding it to IPFS.
    ///
    /// This is a utility function that adds a document to IPFS and returns its CID
    /// without pinning or publishing. Useful for getting document IDs before deciding
    /// whether to publish.
    ///
    /// **Note:** This only adds to IPFS, it doesn't pin or publish. Use `channel::post()`
    /// for the full workflow.
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
    /// Create a new SyncChannel and subscribe to its PubSub topic.
    ///
    /// This is the "subscribe" side of the pub/sub flow. Calling this function:
    /// 1. Creates a channel resource
    /// 2. Subscribes to PubSub topic "doc-sync/{channel_name}"
    /// 3. Registers this module to receive documents (with PR #691)
    ///
    /// **⚠️ REQUIRES PR #691** to receive messages. Without it:
    /// - ✅ Subscription succeeds
    /// - ❌ Messages won't trigger `on_new_doc` events
    ///
    /// ## What PR #691 Does:
    /// When a message arrives on "doc-sync/{channel}":
    /// 1. Host detects "doc-sync/" prefix
    /// 2. Routes to `doc_sync_topic_message_handler()` (not default handler)
    /// 3. Validates message (optional `CatalystSignedDocument` check)
    /// 4. Creates `OnNewDocEvent` with channel name and document data
    /// 5. Dispatches event to all modules subscribed to that channel
    /// 6. Your `on_new_doc` handler is called
    fn new(name: ChannelName) -> SyncChannelImpl {
        info!(target: "doc_sync", "📡 Subscribing to channel: {}", name);

        // Subscribe to PubSub topic for this channel
        // Topic format: "doc-sync/{channel_name}"
        let topic = format!("doc-sync/{}", name);
        match pubsub_subscribe(&topic) {
            Ok(_) => info!(target: "doc_sync", "✓ Subscribed to topic: {}", topic),
            Err(e) => info!(target: "doc_sync", "✗ Failed to subscribe: {:?}", e),
        }

        SyncChannelImpl { name: name.clone() }
    }

    /// Close a SyncChannel and unsubscribe from its topic.
    ///
    /// **Note:** Currently a stub - doesn't actually unsubscribe from PubSub.
    /// A full implementation would call pubsub_unsubscribe().
    fn close(
        &self,
        name: ChannelName,
    ) -> Result<bool, exports::hermes::doc_sync::api::Errno> {
        info!(target: "doc_sync", "Closing channel: {}", name);
        // TODO: Call pubsub_unsubscribe() to stop receiving messages
        Ok(true)
    }

    /// Post a document to IPFS and broadcast it to channel subscribers.
    ///
    /// This is the "publish" side of the pub/sub flow. It executes the complete
    /// 4-step workflow to ensure the document is stored, persisted, and distributed.
    ///
    /// ## 4-Step Workflow:
    ///
    /// ### Step 1: Add to IPFS (file_add)
    /// - Stores document in IPFS network
    /// - Returns CID (Content IDentifier)
    /// - Document is now retrievable by any IPFS node
    ///
    /// ### Step 2: Pin (file_pin)
    /// - Marks document as "important" in local IPFS
    /// - Prevents garbage collection
    /// - Ensures long-term availability
    ///
    /// ### Step 3: Pre-publish (TODO #630)
    /// - Placeholder for validation/signing
    /// - Could add CatalystSignedDocument wrapper
    /// - Could check permissions/quotas
    ///
    /// ### Step 4: Publish to PubSub (pubsub_publish)
    /// - Broadcasts document to topic "doc-sync/{channel}"
    /// - **With PR #691:** Subscribers' `on_new_doc` handlers are triggered
    /// - **Without PR #691:** Message is published but subscribers aren't notified
    ///
    /// ## Returns:
    /// - `Ok(cid_bytes)`: Document CID as bytes
    /// - `Err(errno)`: If any step fails
    fn post(
        &self,
        doc: DocData,
    ) -> Result<Vec<u8>, exports::hermes::doc_sync::api::Errno> {
        info!(target: "doc_sync", "📤 Posting {} bytes to channel: {}", doc.len(), self.name);

        // Step 1: Add document to IPFS (file_add)
        let ipfs_path = file_add(&doc)
            .map_err(|_| exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder)?;
        info!(target: "doc_sync", "✓ Step 1/4: Added to IPFS → {}", ipfs_path);

        // Step 2: Pin the document (file_pin)
        file_pin(&ipfs_path)
            .map_err(|_| exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder)?;
        info!(target: "doc_sync", "✓ Step 2/4: Pinned → {}", ipfs_path);

        // Step 3: Pre-publish validation (TODO #630)
        // TODO: Add document signing, validation, or metadata
        info!(target: "doc_sync", "⏭ Step 3/4: Pre-publish (skipped - TODO #630)");

        // Step 4: Publish to PubSub (pubsub_publish)
        // This broadcasts to all subscribers on "doc-sync/{channel}"
        // With PR #691, this triggers on_new_doc() on subscribed modules
        let topic = format!("doc-sync/{}", self.name);
        pubsub_publish(&topic, &doc)
            .map_err(|_| exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder)?;
        info!(target: "doc_sync", "✓ Step 4/4: Published to PubSub → {}", topic);

        // Extract CID from path and return it
        let cid_str = ipfs_path.strip_prefix("/ipfs/").unwrap_or(&ipfs_path);
        Ok(cid_str.as_bytes().to_vec())
    }

    /// Prove that specific provers have a copy of the document.
    ///
    /// **Note:** Stub implementation - always returns empty proofs.
    /// Not needed for basic pub/sub demo.
    fn prove_includes(
        &self,
        _loc: Vec<u8>,
        _provers: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>, exports::hermes::doc_sync::api::Errno> {
        Ok(vec![])
    }

    /// Prove that specific provers do NOT have a copy of the document.
    ///
    /// **Note:** Stub implementation - always returns empty proofs.
    /// Not needed for basic pub/sub demo.
    fn prove_excludes(
        &self,
        _loc: Vec<u8>,
        _provers: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>, exports::hermes::doc_sync::api::Errno> {
        Ok(vec![])
    }

    /// Retrieve a document from IPFS by its CID.
    ///
    /// **Note:** Stub implementation - always returns error.
    /// Not needed for basic pub/sub demo (use IPFS file_get directly if needed).
    fn get(
        &self,
        _loc: Vec<u8>,
    ) -> Result<Vec<u8>, exports::hermes::doc_sync::api::Errno> {
        Err(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder)
    }
}

/// HTTP Gateway implementation for testing with curl.
///
/// Provides a simple REST endpoint to post documents without writing custom code.
///
/// ## Available Endpoints:
/// - **POST /api/doc-sync/post** - Post a document to the default "documents" channel
///
/// ## Example Usage:
/// ```bash
/// curl -X POST http://localhost:5000/api/doc-sync/post \
///   -H "Host: athena.hermes.local" \
///   -H "Content-Type: text/plain" \
///   -d "Hello, IPFS!"
/// ```
///
/// ## Response:
/// ```json
/// {
///   "success": true,
///   "cid": "bafkreib..."
/// }
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
///
/// This publishes a document through the 4-step workflow: add to IPFS, pin, validate, publish to PubSub.
///
/// **Note:** Publishing to PubSub works now. PR #691 is needed to route incoming PubSub messages
/// to the `on_new_doc` event handler. Without it, documents are published but subscribers don't
/// receive events. See module docs for the complete pub/sub flow.
pub mod channel {
    use super::*;
    use exports::hermes::doc_sync::api::GuestSyncChannel;

    /// Post a document to the "documents" channel.
    ///
    /// Returns the document's CID on success.
    pub fn post(document_bytes: DocData) -> Result<Vec<u8>, exports::hermes::doc_sync::api::Errno> {
        let channel = SyncChannelImpl {
            name: "documents".to_string(),
        };
        channel.post(document_bytes)
    }
}
