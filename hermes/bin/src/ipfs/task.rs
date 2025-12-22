//! IPFS Task
use std::{collections::HashSet, str::FromStr};

use hermes_ipfs::{
    AddIpfsFile, Cid, HermesIpfs, IpfsPath as PathIpfsFile, PeerId as TargetPeerId,
    doc_sync::payload::{self, DocumentDisseminationBody},
    rust_ipfs::path::PathRoot,
};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

use super::HERMES_IPFS;
use crate::{
    event::{HermesEvent, queue::send},
    runtime_extensions::{
        bindings::hermes::ipfs::api::{
            DhtKey, DhtValue, Errno, MessageData, PeerId, PubsubMessage, PubsubTopic,
        },
        hermes::{doc_sync::OnNewDocEvent, ipfs::event::OnTopicEvent},
    },
};

/// Chooses how subscription messages are handled.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) enum SubscriptionKind {
    /// Handle messages as regular ipfs messages.
    #[default]
    Default,
    /// Handle messages as Doc Sync messages.
    DocSync,
}

/// IPFS Command
pub(crate) enum IpfsCommand {
    /// Add a new IPFS file
    AddFile(AddIpfsFile, oneshot::Sender<Result<PathIpfsFile, Errno>>),
    /// Get a file from IPFS
    GetFile(PathIpfsFile, oneshot::Sender<Result<Vec<u8>, Errno>>),
    /// Pin a file
    PinFile(Cid, oneshot::Sender<Result<bool, Errno>>),
    /// Un-pin a file
    UnPinFile(Cid, oneshot::Sender<Result<bool, Errno>>),
    /// Get DHT value
    GetDhtValue(DhtKey, oneshot::Sender<Result<DhtValue, Errno>>),
    /// Put DHT value
    PutDhtValue(DhtKey, DhtValue, oneshot::Sender<Result<bool, Errno>>),
    /// Provide a DHT value
    DhtProvide(DhtKey, oneshot::Sender<Result<(), Errno>>),
    /// Get providers of a DHT value
    DhtGetProviders(
        DhtKey,
        oneshot::Sender<Result<HashSet<hermes_ipfs::PeerId>, Errno>>,
    ),
    /// Publish to a topic
    Publish(PubsubTopic, MessageData, oneshot::Sender<Result<(), Errno>>),
    /// Subscribe to a topic
    Subscribe(
        PubsubTopic,
        SubscriptionKind,
        oneshot::Sender<Result<JoinHandle<()>, Errno>>,
    ),
    /// Evict Peer from node
    EvictPeer(PeerId, oneshot::Sender<Result<bool, Errno>>),
    /// Gets the peer identity
    Identity(
        Option<PeerId>,
        oneshot::Sender<Result<hermes_ipfs::PeerInfo, Errno>>,
    ),
}

/// Handle IPFS commands in asynchronous task.
#[allow(clippy::too_many_lines)]
pub(crate) async fn ipfs_command_handler(
    hermes_node: HermesIpfs,
    mut queue_rx: mpsc::Receiver<IpfsCommand>,
) -> anyhow::Result<()> {
    tracing::info!("🎬 ipfs_command_handler started");
    while let Some(ipfs_command) = queue_rx.recv().await {
        tracing::debug!("📨 Received command: {:?}", std::mem::discriminant(&ipfs_command));
        match ipfs_command {
            IpfsCommand::AddFile(ipfs_file, tx) => {
                let response = hermes_node
                    .add_ipfs_file(ipfs_file)
                    .await
                    .map_err(|_| Errno::FileAddError);
                send_response(response, tx);
            },
            IpfsCommand::GetFile(ipfs_path, tx) => {
                let response = hermes_node
                    .get_ipfs_file(ipfs_path.into())
                    .await
                    .map_err(|_| Errno::FileGetError);
                send_response(response, tx);
            },
            IpfsCommand::PinFile(cid, tx) => {
                tracing::info!("📍 Processing PinFile command for CID: {}", cid);
                let response = match hermes_node.insert_pin(&cid).await {
                    Ok(()) => {
                        tracing::info!("✅ Pin succeeded for CID: {}", cid);
                        Ok(true)
                    },
                    Err(err) if err.to_string().contains("already pinned recursively") => {
                        tracing::debug!(cid = %cid, "file already pinned");
                        Ok(true)
                    },
                    Err(err) => {
                        tracing::error!(cid = %cid, "failed to pin: {}", err);
                        Ok(false)
                    },
                };
                tracing::info!("📤 Sending response for PinFile: {:?}", response);
                send_response(response, tx);
            },
            IpfsCommand::UnPinFile(cid, tx) => {
                let response = match hermes_node.remove_pin(&cid).await {
                    Ok(()) => Ok(true),
                    Err(err) => {
                        tracing::error!(cid = %cid, "failed to un-pin: {}", err);
                        Ok(false)
                    },
                };
                send_response(response, tx);
            },
            IpfsCommand::GetDhtValue(key, tx) => {
                let response = hermes_node.dht_get(key.clone()).await.map_err(|err| {
                    tracing::error!(dht_key = ?key, "failed to get DHT value: {}", err);
                    Errno::DhtGetError
                });
                send_response(response, tx);
            },
            IpfsCommand::PutDhtValue(key, value, tx) => {
                let response = hermes_node.dht_put(key, value).await.is_ok();
                send_response(Ok(response), tx);
            },
            IpfsCommand::Publish(topic, message, tx) => {
                let result = hermes_node
                    .pubsub_publish(&topic, message)
                    .await
                    .map_err(|e| {
                        tracing::error!(topic = %topic, "pubsub_publish failed: {}", e);
                        Errno::PubsubPublishError
                    });
                send_response(result, tx);
            },
            IpfsCommand::Subscribe(topic, kind, tx) => {
                let stream = hermes_node
                    .pubsub_subscribe(&topic)
                    .await
                    .map_err(|_| Errno::PubsubSubscribeError)?;

                let message_handler = TopicMessageHandler::new(&topic, match kind {
                    SubscriptionKind::Default => topic_message_handler,
                    SubscriptionKind::DocSync => doc_sync_topic_message_handler,
                });

                let subscription_handler =
                    TopicSubscriptionStatusHandler::new(&topic, topic_subscription_handler);
                let handle = hermes_ipfs::subscription_stream_task(
                    stream,
                    move |msg| {
                        message_handler.handle(msg);
                    },
                    move |msg| {
                        subscription_handler.handle(msg);
                    },
                );
                send_response(Ok(handle), tx);
            },
            IpfsCommand::EvictPeer(peer, tx) => {
                let peer_id = TargetPeerId::from_str(&peer).map_err(|_| Errno::InvalidPeerId)?;
                let status = hermes_node.ban_peer(peer_id).await.is_ok();
                send_response(Ok(status), tx);
            },
            IpfsCommand::DhtProvide(key, tx) => {
                let response = hermes_node.dht_provide(key.clone()).await.map_err(|err| {
                    tracing::error!(dht_key = ?key, "DHT provide failed: {}", err);
                    Errno::DhtProvideError
                });
                send_response(response, tx);
            },
            IpfsCommand::DhtGetProviders(key, tx) => {
                let response = hermes_node
                    .dht_get_providers(key.clone())
                    .await
                    .map_err(|err| {
                        tracing::error!(dht_key = ?key, "DHT get providers failed: {}", err);
                        Errno::DhtGetProvidersError
                    });
                send_response(response, tx);
            },
            IpfsCommand::Identity(peer_id, tx) => {
                let peer_id = match peer_id {
                    Some(peer_id) => {
                        Some(
                            hermes_ipfs::PeerId::from_str(&peer_id)
                                .map_err(|_| Errno::InvalidPeerId)?,
                        )
                    },
                    None => None,
                };

                let response = hermes_node.identity(peer_id).await.map_err(|err| {
                    tracing::error!(peer_id = ?peer_id, "Identity failed: {}", err);
                    Errno::GetPeerIdError
                });

                send_response(response, tx);
            },
        }
    }
    hermes_node.stop().await;
    Ok(())
}

/// A handler for messages from the IPFS pubsub topic
pub(super) struct TopicMessageHandler<T>
where T: Fn(hermes_ipfs::rust_ipfs::GossipsubMessage, String) + Send + Sync + 'static
{
    /// The topic.
    topic: String,

    /// The handler implementation.
    callback: T,
}

impl<T> TopicMessageHandler<T>
where T: Fn(hermes_ipfs::rust_ipfs::GossipsubMessage, String) + Send + Sync + 'static
{
    /// Creates the new handler.
    pub fn new(
        topic: &impl ToString,
        callback: T,
    ) -> Self {
        Self {
            topic: topic.to_string(),
            callback,
        }
    }

    /// Forwards the message to the handler.
    pub fn handle(
        &self,
        msg: hermes_ipfs::rust_ipfs::GossipsubMessage,
    ) {
        (self.callback)(msg, self.topic.clone());
    }
}

/// A handler for subscribe/unsubscribe events from the IPFS pubsub topic
pub(super) struct TopicSubscriptionStatusHandler<T>
where T: Fn(hermes_ipfs::SubscriptionStatusEvent, String) + Send + Sync + 'static
{
    /// The topic.
    topic: String,

    /// The handler implementation.
    callback: T,
}

impl<T> TopicSubscriptionStatusHandler<T>
where T: Fn(hermes_ipfs::SubscriptionStatusEvent, String) + Send + Sync + 'static
{
    /// Creates the new handler.
    pub fn new(
        topic: &impl ToString,
        callback: T,
    ) -> Self {
        Self {
            topic: topic.to_string(),
            callback,
        }
    }

    /// Passes the subscription event to the handler.
    pub fn handle(
        &self,
        subscription_event: hermes_ipfs::SubscriptionStatusEvent,
    ) {
        (self.callback)(subscription_event, self.topic.clone());
    }
}

/// Handler function for topic message streams.
fn topic_message_handler(
    message: hermes_ipfs::rust_ipfs::GossipsubMessage,
    topic: String,
) {
    if let Some(ipfs) = HERMES_IPFS.get() {
        let app_names = ipfs.apps.subscribed_apps(SubscriptionKind::Default, &topic);
        let on_topic_event = OnTopicEvent {
            message: PubsubMessage {
                topic,
                message: message.data.into(),
                publisher: message.source.map(|p| p.to_string()),
            },
        };
        // Dispatch Hermes Event
        if let Err(err) = send(HermesEvent::new(
            on_topic_event.clone(),
            crate::event::TargetApp::List(app_names),
            crate::event::TargetModule::All,
        )) {
            tracing::error!(
                on_topic_event = %on_topic_event,
                err = err.to_string(),
                "Failed to send on_topic_event.",
            );
        }
    } else {
        tracing::error!("Failed to send on_topic_event. IPFS is uninitialized");
    }
}

/// Handler for Doc Sync PubSub messages on "*.new" topics.
///
/// Receives P2P messages containing CBOR-encoded CID lists, spawns an async task
/// to fetch document content from IPFS, and dispatches OnNewDocEvent to subscribed apps.
///
/// Uses async file operations (file_get_async) to avoid blocking the PubSub handler.
/// Message format: payload::New → DocumentDisseminationBody::Docs { docs: Vec<Cid> }
#[allow(
    clippy::needless_pass_by_value,
    reason = "The event will be eventually consumed in the handler"
)]
fn doc_sync_topic_message_handler(
    message: hermes_ipfs::rust_ipfs::GossipsubMessage,
    topic: String,
) {
    tracing::info!("🔔 doc_sync_topic_message_handler called! topic={}, message_len={}", topic, message.data.len());
    if let Ok(msg_str) = std::str::from_utf8(&message.data) {
        tracing::info!("RECEIVED PubSub message on topic: {} - data: {}", topic, msg_str);
    } else {
        tracing::info!("RECEIVED PubSub message on topic: {}", topic);
    }

    // TODO: match the topic against a static list.
    let Some(channel_name) = topic.strip_suffix(".new") else {
        tracing::error!("Handling an IPFS message on a wrong channel.");
        return;
    };

    let payload = match minicbor::decode::<payload::New>(&message.data) {
        Ok(payload) => DocumentDisseminationBody::from(payload),
        Err(err) => {
            tracing::error!(%channel_name, %err, "❌ Failed to decode .new payload from IPFS message");
            return;
        },
    };
    tracing::info!("✅ Decoded payload successfully");

    let new_cids = match payload {
        DocumentDisseminationBody::Docs { docs, .. } => docs,
        DocumentDisseminationBody::Manifest { .. } => {
            tracing::error!("❌ Manifest is not supported in a .new payload");
            return;
        },
    };
    tracing::info!("✅ Extracted {} CIDs from payload", new_cids.len());
    for cid in &new_cids {
        tracing::info!("RECEIVED PubSub message with CID: {}", cid);
    }

    let channel_name_owned = channel_name.to_string();
    let topic_owned = topic.clone();

    tracing::info!("🚀 Spawning async task to process {} CIDs", new_cids.len());

    // Spawn async task to avoid blocking PubSub handler during file operations
    tokio::spawn(async move {
        tracing::info!("📥 Inside spawned task, processing {} CIDs", new_cids.len());
        let Some(ipfs) = HERMES_IPFS.get() else {
            tracing::error!("❌ IPFS global instance is uninitialized");
            return;
        };
        tracing::info!("✅ Got HERMES_IPFS instance");

        let mut contents = Vec::with_capacity(new_cids.len());

        for cid in new_cids {
            tracing::info!("🔄 Processing CID: {}", cid);

            // IMPORTANT: The message contains dag-cbor CIDs (CIDv1, codec 0x51), but IPFS storage
            // uses dag-pb CIDv0 (codec 0x70). We need to convert to CIDv0 before fetching.
            // Both CIDs have the same multihash, so we can reconstruct the CIDv0.
            let storage_cid = hermes_ipfs::Cid::new_v0(*cid.hash())
                .map_err(|e| {
                    tracing::error!("Failed to convert CID to v0: {}", e);
                    e
                })
                .ok();

            if storage_cid.is_none() {
                tracing::error!("Failed to convert CID {} to CIDv0, skipping", cid);
                continue;
            }
            let storage_cid = storage_cid.unwrap();
            let path = hermes_ipfs::IpfsPath::new(PathRoot::Ipld(storage_cid)).to_string();

            tracing::info!("📥 Fetching content (protocol CID: {}, storage CID: {})", cid, storage_cid);
            let content = match ipfs.file_get_async(&path).await {
                Ok(ipfs_file) => {
                    tracing::info!("✅ Got content ({} bytes) for CID: {}", ipfs_file.len(), cid);
                    // Log content for test detection
                    if let Ok(content_str) = std::str::from_utf8(&ipfs_file) {
                        tracing::info!("RECEIVED PubSub message content: {}", content_str);
                    }
                    ipfs_file
                },
                Err(err) => {
                    tracing::error!(
                        %channel_name_owned, %cid, %err,
                        "❌ Failed to get content of the document after a successful IPFS pin"
                    );
                    continue;
                },
            };

            contents.push(content);
        }
        tracing::info!("✅ Finished processing all CIDs, {} contents collected", contents.len());

        let app_names = ipfs.apps.subscribed_apps(SubscriptionKind::DocSync, &topic_owned);

        for content in contents {
            let event = HermesEvent::new(
                OnNewDocEvent::new(&channel_name_owned, &content),
                crate::event::TargetApp::List(app_names.clone()),
                crate::event::TargetModule::All,
            );

            // Dispatch Hermes Event
            if let Err(err) = send(event) {
                tracing::error!(%channel_name_owned, %err, "Failed to send `on_new_doc` event");
            }
        }
    });
}

/// Handler for the subscription events for topic
#[allow(clippy::needless_pass_by_value)] // The event will be eventually consumed in the handler
fn topic_subscription_handler(
    subscription_event: hermes_ipfs::SubscriptionStatusEvent,
    topic: String,
) {
    tracing::trace!(%subscription_event, %topic, "Subscription event");
}

/// Send the response of the IPFS command
fn send_response<T>(
    response: T,
    tx: oneshot::Sender<T>,
) {
    if tx.send(response).is_err() {
        tracing::error!("sending IPFS command response should not fail");
    }
}
