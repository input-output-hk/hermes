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
    utils::log::{self, error, info, warn},
};

use exports::hermes::doc_sync::api::GuestSyncChannel;
use hermes::http_gateway::api::{Bstr, Headers, HttpGatewayResponse, HttpResponse};

// Removed unused import
use hermes_ipfs::{AddIpfsFile, Cid, HermesIpfs};
use std::sync::OnceLock;

#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::Runtime;

/// Doc Sync component.
struct Component;

impl exports::hermes::init::event::Guest for Component {
    fn init() -> bool {
        log::init(log::LevelFilter::Trace);
        info!(target: "doc_sync::init", "💫 Opening channel...");
        let _chan = SyncChannel::new("documents");
        info!(target: "doc_sync::init", "💫 Channel opened");
        true
    }
}

impl exports::hermes::doc_sync::event::Guest for Component {
    fn on_new_doc(
        channel: ChannelName,
        doc: DocData,
    ) {
        log::init(log::LevelFilter::Trace);
        info!(target: "doc_sync::on_new_doc", "Received new document on channel: {}, doc_byte_length: {}", channel, doc.len());
    }
}

/// Global IPFS instance
static IPFS_INSTANCE: OnceLock<HermesIpfs> = OnceLock::new();

/// Global runtime for async operations (non-WASM only)
#[cfg(not(target_arch = "wasm32"))]
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Internal representation of the SyncChannel resource
pub struct SyncChannelImpl {
    name: ChannelName,
}

/// Initialize IPFS instance and runtime
#[cfg(not(target_arch = "wasm32"))]
fn get_or_init_ipfs() -> &'static HermesIpfs {
    IPFS_INSTANCE.get_or_init(|| {
        let rt = get_or_init_runtime();
        rt.block_on(async {
            HermesIpfs::start()
                .await
                .expect("Failed to start IPFS node")
        })
    })
}

/// Initialize IPFS instance for WASM (without blocking)
#[cfg(target_arch = "wasm32")]
fn get_or_init_ipfs() -> &'static HermesIpfs {
    IPFS_INSTANCE.get_or_init(|| {
        // In WASM, we use futures::executor::block_on instead of tokio's block_on
        futures::executor::block_on(async {
            HermesIpfs::start()
                .await
                .expect("Failed to start IPFS node")
        })
    })
}

/// Initialize runtime (non-WASM only)
#[cfg(not(target_arch = "wasm32"))]
fn get_or_init_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| Runtime::new().expect("Failed to create tokio runtime"))
}

// Implement the doc-sync API functions
impl exports::hermes::doc_sync::api::Guest for Component {
    type SyncChannel = SyncChannelImpl;

    #[cfg(not(target_arch = "wasm32"))]
    fn id_for(doc: DocData) -> Vec<u8> {
        let ipfs = get_or_init_ipfs();
        let rt = get_or_init_runtime();

        rt.block_on(async {
            // Add document to IPFS to get the actual CID
            let add_file = AddIpfsFile::from(doc);
            match ipfs.add_ipfs_file(add_file).await {
                Ok(ipfs_path) => {
                    // Parse CID from path string
                    let path_str = ipfs_path.to_string();
                    if let Some(cid_str) = path_str.strip_prefix("/ipfs/") {
                        match cid_str.parse::<Cid>() {
                            Ok(cid) => cid.to_string().into_bytes(),
                            Err(_) => {
                                error!(target: "doc_sync::id_for", "Failed to parse CID from path: {}", path_str);
                                format!("bafkreigh2akiscaildcqabsyg3dfr6chu3fgpregiymsck7e7aqa4s52zy").into_bytes()
                            }
                        }
                    } else {
                        error!(target: "doc_sync::id_for", "Path does not start with /ipfs/: {}", path_str);
                        format!("bafkreigh2akiscaildcqabsyg3dfr6chu3fgpregiymsck7e7aqa4s52zy").into_bytes()
                    }
                },
                Err(ipfs_error) => {
                    error!(target: "doc_sync::id_for", "Failed to add document to IPFS: {:?}", ipfs_error);
                    format!("bafkreigh2akiscaildcqabsyg3dfr6chu3fgpregiymsck7e7aqa4s52zy").into_bytes()
                }
            }
        })
    }

    #[cfg(target_arch = "wasm32")]
    fn id_for(doc: DocData) -> Vec<u8> {
        let ipfs = get_or_init_ipfs();

        futures::executor::block_on(async {
            // Add document to IPFS to get the actual CID
            let add_file = AddIpfsFile::from(doc);
            match ipfs.add_ipfs_file(add_file).await {
                Ok(ipfs_path) => {
                    // Parse CID from path string
                    let path_str = ipfs_path.to_string();
                    if let Some(cid_str) = path_str.strip_prefix("/ipfs/") {
                        match cid_str.parse::<Cid>() {
                            Ok(cid) => cid.to_string().into_bytes(),
                            Err(_) => {
                                error!(target: "doc_sync::id_for", "Failed to parse CID from path: {}", path_str);
                                format!("bafkreigh2akiscaildcqabsyg3dfr6chu3fgpregiymsck7e7aqa4s52zy").into_bytes()
                            }
                        }
                    } else {
                        error!(target: "doc_sync::id_for", "Path does not start with /ipfs/: {}", path_str);
                        format!("bafkreigh2akiscaildcqabsyg3dfr6chu3fgpregiymsck7e7aqa4s52zy").into_bytes()
                    }
                },
                Err(ipfs_error) => {
                    error!(target: "doc_sync::id_for", "Failed to add document to IPFS: {:?}", ipfs_error);
                    format!("bafkreigh2akiscaildcqabsyg3dfr6chu3fgpregiymsck7e7aqa4s52zy").into_bytes()
                }
            }
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
        info!(target: "doc_sync::sync_channel", "Closing sync channel: {}", name);
        // In a real implementation, this would clean up the channel
        Ok(true)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn post(
        &self,
        doc: DocData,
    ) -> Result<Vec<u8>, exports::hermes::doc_sync::api::Errno> {
        info!(target: "doc_sync::sync_channel", "Posting document to channel: {}, doc_size: {}", self.name, doc.len());

        let ipfs = get_or_init_ipfs();
        let rt = get_or_init_runtime();

        rt.block_on(async {
            // Step 1: Add document to IPFS
            let add_file = AddIpfsFile::from(doc.clone());
            let ipfs_path = match ipfs.add_ipfs_file(add_file).await {
                Ok(path) => {
                    info!(target: "doc_sync::sync_channel", "Document added to IPFS: {}", path);
                    path
                },
                Err(ipfs_error) => {
                    error!(target: "doc_sync::sync_channel", "Failed to add document to IPFS: {:?}", ipfs_error);
                    return Err(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder);
                },
            };

            // Extract CID from path
            let path_str = ipfs_path.to_string();
            let cid = if let Some(cid_str) = path_str.strip_prefix("/ipfs/") {
                match cid_str.parse::<Cid>() {
                    Ok(cid) => cid,
                    Err(parse_error) => {
                        error!(target: "doc_sync::sync_channel", "Failed to parse CID from path {}: {:?}", path_str, parse_error);
                        return Err(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder);
                    }
                }
            } else {
                error!(target: "doc_sync::sync_channel", "Path does not start with /ipfs/: {}", path_str);
                return Err(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder);
            };

            // Step 2: Pin the document
            match ipfs.insert_pin(&cid).await {
                Ok(()) => {
                    info!(target: "doc_sync::sync_channel", "Document pinned successfully: {}", cid);
                },
                Err(ipfs_error) => {
                    error!(target: "doc_sync::sync_channel", "Failed to pin document: {:?}", ipfs_error);
                    return Err(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder);
                },
            }

            // Step 3: Pre-publish step (placeholder for separate issue #630)
            // TODO: Implement pre-publish step when issue #630 is resolved

            // Step 4: Publish to PubSub
            let pubsub_topic = format!("doc-sync/{}", self.name);
            match ipfs.pubsub_publish(pubsub_topic.clone(), doc).await {
                Ok(()) => {
                    info!(target: "doc_sync::sync_channel",
                          "Document published to PubSub - topic: {}", pubsub_topic);
                },
                Err(ipfs_error) => {
                    error!(target: "doc_sync::sync_channel", "Failed to publish document to PubSub: {:?}", ipfs_error);
                    return Err(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder);
                },
            }

            // Return the CID as bytes
            Ok(cid.to_string().into_bytes())
        })
    }

    #[cfg(target_arch = "wasm32")]
    fn post(
        &self,
        doc: DocData,
    ) -> Result<Vec<u8>, exports::hermes::doc_sync::api::Errno> {
        info!(target: "doc_sync::sync_channel", "Posting document to channel: {}, doc_size: {}", self.name, doc.len());

        let ipfs = get_or_init_ipfs();

        futures::executor::block_on(async {
            // Step 1: Add document to IPFS
            let add_file = AddIpfsFile::from(doc.clone());
            let ipfs_path = match ipfs.add_ipfs_file(add_file).await {
                Ok(path) => {
                    info!(target: "doc_sync::sync_channel", "Document added to IPFS: {}", path);
                    path
                },
                Err(ipfs_error) => {
                    error!(target: "doc_sync::sync_channel", "Failed to add document to IPFS: {:?}", ipfs_error);
                    return Err(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder);
                },
            };

            // Extract CID from path
            let path_str = ipfs_path.to_string();
            let cid = if let Some(cid_str) = path_str.strip_prefix("/ipfs/") {
                match cid_str.parse::<Cid>() {
                    Ok(cid) => cid,
                    Err(parse_error) => {
                        error!(target: "doc_sync::sync_channel", "Failed to parse CID from path {}: {:?}", path_str, parse_error);
                        return Err(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder);
                    }
                }
            } else {
                error!(target: "doc_sync::sync_channel", "Path does not start with /ipfs/: {}", path_str);
                return Err(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder);
            };

            // Step 2: Pin the document
            match ipfs.insert_pin(&cid).await {
                Ok(()) => {
                    info!(target: "doc_sync::sync_channel", "Document pinned successfully: {}", cid);
                },
                Err(ipfs_error) => {
                    error!(target: "doc_sync::sync_channel", "Failed to pin document: {:?}", ipfs_error);
                    return Err(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder);
                },
            }

            // Step 3: Pre-publish step (placeholder for separate issue #630)
            // TODO: Implement pre-publish step when issue #630 is resolved

            // Step 4: Publish to PubSub
            let pubsub_topic = format!("doc-sync/{}", self.name);
            match ipfs.pubsub_publish(pubsub_topic.clone(), doc).await {
                Ok(()) => {
                    info!(target: "doc_sync::sync_channel",
                          "Document published to PubSub - topic: {}", pubsub_topic);
                },
                Err(ipfs_error) => {
                    error!(target: "doc_sync::sync_channel", "Failed to publish document to PubSub: {:?}", ipfs_error);
                    return Err(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder);
                },
            }

            // Return the CID as bytes
            Ok(cid.to_string().into_bytes())
        })
    }

    fn prove_includes(
        &self,
        _loc: Vec<u8>,
        _provers: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>, exports::hermes::doc_sync::api::Errno> {
        // Placeholder implementation
        warn!(target: "doc_sync::sync_channel", "prove_includes not yet implemented");
        Ok(vec![])
    }

    fn prove_excludes(
        &self,
        _loc: Vec<u8>,
        _provers: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>, exports::hermes::doc_sync::api::Errno> {
        // Placeholder implementation
        warn!(target: "doc_sync::sync_channel", "prove_excludes not yet implemented");
        Ok(vec![])
    }

    fn get(
        &self,
        _loc: Vec<u8>,
    ) -> Result<Vec<u8>, exports::hermes::doc_sync::api::Errno> {
        // Placeholder implementation
        warn!(target: "doc_sync::sync_channel", "get not yet implemented");
        Err(exports::hermes::doc_sync::api::Errno::DocErrorPlaceholder)
    }
}

/// HTTP Gateway implementation for doc-sync
impl exports::hermes::http_gateway::event::Guest for Component {
    fn reply(
        body: Vec<u8>,
        _headers: Headers,
        path: String,
        method: String,
    ) -> Option<HttpGatewayResponse> {
        log::init(log::LevelFilter::Trace);
        info!(target: "doc_sync::http", "Processing HTTP request: {} {}", method, path);

        match (method.as_str(), path.as_str()) {
            ("POST", "/api/doc-sync/post") => handle_post_document(body),
            ("GET", "/api/doc-sync/health") => handle_health_check(),
            ("POST", path) if path.starts_with("/api/doc-sync/channel/") => {
                // Extract channel name from path like /api/doc-sync/channel/{name}/post
                if let Some(channel_name) = extract_channel_name(path) {
                    handle_channel_post(body, channel_name)
                } else {
                    create_error_response(400, "Invalid channel path")
                }
            },
            _ => create_error_response(404, "Endpoint not found"),
        }
    }
}

/// Handle POST /api/doc-sync/post - Posts document using default channel
fn handle_post_document(body: Vec<u8>) -> Option<HttpGatewayResponse> {
    if body.is_empty() {
        return create_error_response(400, "Request body cannot be empty");
    }

    info!(target: "doc_sync::http", "Posting document with size: {} bytes", body.len());

    match channel::post(body) {
        Ok(cid_bytes) => {
            let cid_string = String::from_utf8_lossy(&cid_bytes);
            info!(target: "doc_sync::http", "Document posted successfully with CID: {}", cid_string);

            let response_body = serde_json::json!({
                "success": true,
                "cid": cid_string.to_string(),
                "message": "Document posted to IPFS and published to PubSub"
            })
            .to_string();

            Some(HttpGatewayResponse::Http(HttpResponse {
                code: 200,
                headers: vec![(
                    "content-type".to_string(),
                    vec!["application/json".to_string()],
                )],
                body: Bstr::from(response_body),
            }))
        },
        Err(e) => {
            error!(target: "doc_sync::http", "Failed to post document: {:?}", e);
            create_error_response(500, "Failed to post document to IPFS")
        },
    }
}

/// Handle POST /api/doc-sync/channel/{name}/post - Posts document to specific channel
fn handle_channel_post(
    body: Vec<u8>,
    channel_name: String,
) -> Option<HttpGatewayResponse> {
    if body.is_empty() {
        return create_error_response(400, "Request body cannot be empty");
    }

    info!(target: "doc_sync::http", "Posting document to channel '{}' with size: {} bytes", channel_name, body.len());

    let channel_impl = SyncChannelImpl {
        name: channel_name.clone(),
    };
    match channel_impl.post(body) {
        Ok(cid_bytes) => {
            let cid_string = String::from_utf8_lossy(&cid_bytes);
            info!(target: "doc_sync::http", "Document posted successfully to channel '{}' with CID: {}", channel_name, cid_string);

            let response_body = serde_json::json!({
                "success": true,
                "cid": cid_string.to_string(),
                "channel": channel_name,
                "message": "Document posted to IPFS and published to PubSub channel"
            })
            .to_string();

            Some(HttpGatewayResponse::Http(HttpResponse {
                code: 200,
                headers: vec![(
                    "content-type".to_string(),
                    vec!["application/json".to_string()],
                )],
                body: Bstr::from(response_body),
            }))
        },
        Err(e) => {
            error!(target: "doc_sync::http", "Failed to post document to channel '{}': {:?}", channel_name, e);
            create_error_response(
                500,
                &format!("Failed to post document to channel '{}':", channel_name),
            )
        },
    }
}

/// Handle GET /api/doc-sync/health - Health check endpoint
fn handle_health_check() -> Option<HttpGatewayResponse> {
    let response_body = serde_json::json!({
        "status": "healthy",
        "service": "doc-sync",
        "version": "0.1.0",
        "endpoints": {
            "POST /api/doc-sync/post": "Post document to default channel",
            "POST /api/doc-sync/channel/{name}/post": "Post document to specific channel",
            "GET /api/doc-sync/health": "Health check"
        }
    })
    .to_string();

    Some(HttpGatewayResponse::Http(HttpResponse {
        code: 200,
        headers: vec![(
            "content-type".to_string(),
            vec!["application/json".to_string()],
        )],
        body: Bstr::from(response_body),
    }))
}

/// Extract channel name from path like /api/doc-sync/channel/{name}/post
fn extract_channel_name(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 5
        && parts[1] == "api"
        && parts[2] == "doc-sync"
        && parts[3] == "channel"
        && parts.get(5) == Some(&"post")
    {
        Some(parts[4].to_string())
    } else {
        None
    }
}

/// Create an error response
fn create_error_response(
    code: u16,
    message: &str,
) -> Option<HttpGatewayResponse> {
    let response_body = serde_json::json!({
        "success": false,
        "error": message
    })
    .to_string();

    Some(HttpGatewayResponse::Http(HttpResponse {
        code,
        headers: vec![(
            "content-type".to_string(),
            vec!["application/json".to_string()],
        )],
        body: Bstr::from(response_body),
    }))
}

/// Channel API as requested in GitHub issue #628
///
/// Apps should be able to call: `let cid = channel::post(document_bytes);`
pub mod channel {
    use super::*;
    use exports::hermes::doc_sync::api::GuestSyncChannel;

    /// Posts a document to IPFS PubSub channel
    ///
    /// This matches the exact API requested in the GitHub issue.
    ///
    /// # Arguments
    /// * `document_bytes` - The raw document data to post
    ///
    /// # Returns
    /// * Content ID (CID) of the posted document on success
    pub fn post(document_bytes: DocData) -> Result<Vec<u8>, exports::hermes::doc_sync::api::Errno> {
        // Create a channel instance and use its post functionality
        let channel_impl = SyncChannelImpl {
            name: "documents".to_string(),
        };
        channel_impl.post(document_bytes)
    }
}
