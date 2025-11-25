#![allow(missing_docs)]
//! Doc Sync Module

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

/// Doc Sync component.
struct Component;

impl exports::hermes::init::event::Guest for Component {
    fn init() -> bool {
        log::init(log::LevelFilter::Trace);
        info!(target: "doc_sync::init", "Opening channel...");
        let _chan = SyncChannel::new("documents");
        info!(target: "doc_sync::init", "Channel opened");
        true
    }
}

/// Event handler triggered when a new document arrives on a subscribed PubSub channel.
///
/// ## Pub/Sub Flow Integration (with PR #691):
///
/// **Publishing side:**
/// 1. App calls `channel::post(doc)` → publishes to PubSub topic `doc-sync/{channel}`
///
/// **Subscribing side:**
/// 1. App calls `SyncChannel::new("channel-name")` → subscribes to topic `doc-sync/channel-name`
/// 2. Host IPFS layer receives PubSub message (via PR #691's `doc_sync_topic_message_handler`)
/// 3. Host validates message (using `CatalystSignedDocument` if configured)
/// 4. Host triggers `on_new_doc` event on all subscribed modules
/// 5. This handler receives the document
///
/// ## Implementation Notes:
/// - Subscription happens automatically when `SyncChannel::new()` is called during `init()`
/// - PR #691 adds infrastructure to route PubSub messages to this handler
/// - Documents can be validated, stored, or processed here
impl exports::hermes::doc_sync::event::Guest for Component {
    fn on_new_doc(
        channel: ChannelName,
        doc: DocData,
    ) {
        log::init(log::LevelFilter::Trace);
        info!(target: "doc_sync", "📥 Received doc on channel '{}': {} bytes", channel, doc.len());

        // TODO: Process received document
        // - Validate signature (CatalystSignedDocument)
        // - Store in local database
        // - Trigger application workflows
        // - Send acknowledgment
    }
}

/// Internal representation of the SyncChannel resource
pub struct SyncChannelImpl {
    name: ChannelName,
}

// Implement the doc-sync API functions
impl exports::hermes::doc_sync::api::Guest for Component {
    type SyncChannel = SyncChannelImpl;

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
    fn new(name: ChannelName) -> SyncChannelImpl {
        info!(target: "doc_sync", "📡 Subscribing to channel: {}", name);

        // Subscribe to PubSub topic for this channel
        // Topic format: "doc-sync/{channel_name}"
        // With PR #691, the host will:
        // 1. Register this as a DocSync subscription (SubscriptionKind::DocSync)
        // 2. Route incoming messages to doc_sync_topic_message_handler()
        // 3. Validate messages (CatalystSignedDocument) if configured
        // 4. Trigger on_new_doc() event when messages arrive
        let topic = format!("doc-sync/{}", name);
        match pubsub_subscribe(&topic) {
            Ok(_) => info!(target: "doc_sync", "✓ Subscribed to topic: {}", topic),
            Err(e) => info!(target: "doc_sync", "✗ Failed to subscribe: {:?}", e),
        }

        SyncChannelImpl { name: name.clone() }
    }

    fn close(
        &self,
        name: ChannelName,
    ) -> Result<bool, exports::hermes::doc_sync::api::Errno> {
        info!(target: "doc_sync", "Closing channel: {}", name);
        Ok(true)
    }

    fn post(
        &self,
        doc: DocData,
    ) -> Result<Vec<u8>, exports::hermes::doc_sync::api::Errno> {
        info!(target: "doc_sync", "Posting {} bytes to channel: {}", doc.len(), self.name);

        // Step 1: Add document to IPFS (file_add)
        let ipfs_path = file_add(&doc)
            .map_err(|_| exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder)?;
        info!(target: "doc_sync", "✓ Added to IPFS: {}", ipfs_path);

        // Step 2: Pin the document (file_pin)
        file_pin(&ipfs_path)
            .map_err(|_| exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder)?;
        info!(target: "doc_sync", "✓ Pinned: {}", ipfs_path);

        // Step 3: Pre-publish step (placeholder for separate issue #630)
        // TODO: Implement pre-publish step when issue #630 is resolved

        // Step 4: Publish to PubSub (pubsub_publish)
        let topic = format!("doc-sync/{}", self.name);
        pubsub_publish(&topic, &doc)
            .map_err(|_| exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder)?;
        info!(target: "doc_sync", "✓ Published to PubSub topic: {}", topic);

        // Extract CID from path and return it
        let cid_str = ipfs_path.strip_prefix("/ipfs/").unwrap_or(&ipfs_path);
        Ok(cid_str.as_bytes().to_vec())
    }

    fn prove_includes(
        &self,
        _loc: Vec<u8>,
        _provers: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>, exports::hermes::doc_sync::api::Errno> {
        Ok(vec![])
    }

    fn prove_excludes(
        &self,
        _loc: Vec<u8>,
        _provers: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>, exports::hermes::doc_sync::api::Errno> {
        Ok(vec![])
    }

    fn get(
        &self,
        _loc: Vec<u8>,
    ) -> Result<Vec<u8>, exports::hermes::doc_sync::api::Errno> {
        Err(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder)
    }
}

/// HTTP Gateway implementation
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
            ("POST", "/api/doc-sync/post") => match channel::post(body) {
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
            },
            _ => json_response(404, serde_json::json!({"error": "Not found"})),
        }
    }
}

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

/// # Document Sync Channel API
///
/// ## Complete Pub/Sub Flow with PR #691:
///
/// ### 1. Subscribe to a channel (App B):
/// ```rust,ignore
/// // In init(), subscribe to receive documents
/// let _channel = SyncChannel::new("documents");
/// // → Calls pubsub_subscribe("doc-sync/documents")
/// // → Host registers DocSync subscription (PR #691)
/// // → Starts listening for messages on this topic
/// ```
///
/// ### 2. Publish to the channel (App A):
/// ```rust,ignore
/// let cid = channel::post(b"Hello, IPFS!")?;
/// // → Executes 4-step workflow (add, pin, validate, publish)
/// // → Publishes to PubSub topic "doc-sync/documents"
/// ```
///
/// ### 3. Receive the document (App B):
/// ```rust,ignore
/// // on_new_doc is automatically triggered by PR #691 infrastructure
/// fn on_new_doc(channel: ChannelName, doc: DocData) {
///     // channel = "documents"
///     // doc = b"Hello, IPFS!"
///     process_document(doc);
/// }
/// ```
///
/// ## PR #691 Integration Details:
/// - Host detects subscription is DocSync type (from topic prefix "doc-sync/")
/// - Routes to `doc_sync_topic_message_handler()` instead of default handler
/// - Validates using `CatalystSignedDocument` if configured
/// - Dispatches `OnNewDocEvent` to all subscribed modules
pub mod channel {
    use super::*;

    /// Posts a document to the default IPFS PubSub channel.
    ///
    /// ## Workflow:
    /// 1. Add document to IPFS → Get CID
    /// 2. Pin document → Ensure persistence
    /// 3. Pre-publish validation (TODO #630)
    /// 4. Publish to PubSub → Notify subscribers
    ///
    /// ## Example:
    /// ```rust,ignore
    /// match channel::post(b"Hello, world!".to_vec()) {
    ///     Ok(cid) => println!("Published: {}", String::from_utf8_lossy(&cid)),
    ///     Err(e) => eprintln!("Error: {:?}", e),
    /// }
    /// ```
    pub fn post(document_bytes: DocData) -> Result<Vec<u8>, exports::hermes::doc_sync::api::Errno> {
        let channel = SyncChannelImpl {
            name: "documents".to_string(),
        };
        channel.post(document_bytes)
    }
}
