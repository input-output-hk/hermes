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

use hermes_ipfs::{AddIpfsFile, Cid, HermesIpfs};

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

impl exports::hermes::doc_sync::event::Guest for Component {
    fn on_new_doc(
        channel: ChannelName,
        doc: DocData,
    ) {
        log::init(log::LevelFilter::Trace);
        info!(target: "doc_sync::on_new_doc", "Received new document on channel: {}, size: {} bytes", channel, doc.len());
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
        futures::executor::block_on(async {
            let ipfs = HermesIpfs::start().await.expect("Failed to start IPFS");
            let add_file = AddIpfsFile::from(doc);
            let ipfs_path = ipfs
                .add_ipfs_file(add_file)
                .await
                .expect("Failed to add file");
            let path_string = ipfs_path.to_string();
            let cid_str = path_string.strip_prefix("/ipfs/").expect("Invalid path");
            cid_str.as_bytes().to_vec()
        })
    }
}

// Implement the SyncChannel resource
impl exports::hermes::doc_sync::api::GuestSyncChannel for SyncChannelImpl {
    fn new(name: ChannelName) -> SyncChannelImpl {
        info!(target: "doc_sync::sync_channel", "Opening sync channel: {}", name);
        // Create the resource - in real implementation, this would set up the channel
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

        futures::executor::block_on(async {
            let ipfs = HermesIpfs::start().await.expect("Failed to start IPFS");

            // Step 1: Add document to IPFS (file_add)
            let add_file = AddIpfsFile::from(doc.clone());
            let ipfs_path = ipfs
                .add_ipfs_file(add_file)
                .await
                .map_err(|_| exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder)?;
            info!(target: "doc_sync", "✓ Added to IPFS: {}", ipfs_path);

            // Extract CID from path
            let path_string = ipfs_path.to_string();
            let cid_str = path_string
                .strip_prefix("/ipfs/")
                .ok_or(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder)?;
            let cid: Cid = cid_str
                .parse()
                .map_err(|_| exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder)?;

            // Step 2: Pin the document (file_pin)
            ipfs.insert_pin(&cid)
                .await
                .map_err(|_| exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder)?;
            info!(target: "doc_sync", "✓ Pinned: {}", cid);

            // Step 3: Pre-publish step (placeholder for separate issue #630)
            // TODO: Implement pre-publish step when issue #630 is resolved

            // Step 4: Publish to PubSub (pubsub_publish)
            let topic = format!("doc-sync/{}", self.name);
            ipfs.pubsub_publish(topic.clone(), doc)
                .await
                .map_err(|_| exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder)?;
            info!(target: "doc_sync", "✓ Published to PubSub topic: {}", topic);

            Ok(cid.to_string().into_bytes())
        })
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

/// HTTP Gateway implementation - for curl demo
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

/// Simple API: `let cid = channel::post(document_bytes);`
pub mod channel {
    use super::*;
    use exports::hermes::doc_sync::api::GuestSyncChannel;

    /// Posts a document to IPFS PubSub channel
    /// Demonstrates the 4-step workflow:
    /// 1. Add to IPFS (file_add)
    /// 2. Pin document (file_pin)
    /// 3. Pre-publish step (TODO)
    /// 4. Publish to PubSub (pubsub_publish)
    pub fn post(document_bytes: DocData) -> Result<Vec<u8>, exports::hermes::doc_sync::api::Errno> {
        let channel = SyncChannelImpl {
            name: "documents".to_string(),
        };
        channel.post(document_bytes)
    }
}
