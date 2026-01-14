//! IPFS Task
use std::{collections::HashSet, str::FromStr, sync::Arc, time::Duration};

use hermes_ipfs::{
    Cid, HermesIpfs, IpfsPath as PathIpfsFile, PeerId as TargetPeerId,
    doc_sync::payload::{self, DocumentDisseminationBody},
};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::timeout,
};

use super::HERMES_IPFS;
use crate::{
    runtime_extensions::{
        bindings::hermes::ipfs::api::{
            DhtKey, DhtValue, Errno, IpfsFile, MessageData, PeerId, PubsubMessage, PubsubTopic,
        },
        hermes::{doc_sync::OnNewDocEvent, ipfs::event::OnTopicEvent},
    },
    wasm::module::ModuleId,
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
    AddFile(IpfsFile, oneshot::Sender<Result<PathIpfsFile, Errno>>),
    /// Get a file from IPFS by CID
    GetFile(Cid, oneshot::Sender<Result<IpfsFile, Errno>>),
    /// Get a file from IPFS by CID with specific providers
    GetFileWithProviders(
        Cid,
        Vec<hermes_ipfs::PeerId>,
        oneshot::Sender<Result<IpfsFile, Errno>>,
    ),
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
        Option<Vec<ModuleId>>,
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
    // Wrap in Arc to allow sharing across spawned tasks
    let hermes_node = Arc::new(hermes_node);

    while let Some(ipfs_command) = queue_rx.recv().await {
        tracing::debug!(
            "Received command: {:?}",
            std::mem::discriminant(&ipfs_command)
        );
        match ipfs_command {
            IpfsCommand::AddFile(ipfs_file, tx) => {
                let response = hermes_node
                    .add_ipfs_file(ipfs_file)
                    .await
                    .map_err(|_| Errno::FileAddError);
                send_response(response, tx);
            },
            IpfsCommand::GetFile(cid, tx) => {
                let response = hermes_node
                    .get_ipfs_file_cbor(&cid)
                    .await
                    .map_err(|_| Errno::FileGetError);
                send_response(response, tx);
            },
            IpfsCommand::GetFileWithProviders(cid, providers, tx) => {
                // Spawn task to avoid blocking the command handler
                // This allows concurrent file fetches and retry logic to work
                let node = Arc::clone(&hermes_node);
                tokio::spawn(async move {
                    let response = node
                        .get_ipfs_file_cbor_with_providers(&cid, &providers)
                        .await
                        .map_err(|_| Errno::FileGetError);
                    send_response(response, tx);
                });
            },
            IpfsCommand::PinFile(cid, tx) => {
                let response = match hermes_node.insert_pin(&cid).await {
                    Ok(()) => {
                        tracing::info!("Pin succeeded for CID: {}", cid.to_string());
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
                tracing::info!("Sending response for PinFile: {:?}", response);
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
            IpfsCommand::Subscribe(topic, kind, module_ids, tx) => {
                let stream = hermes_node
                    .pubsub_subscribe(&topic)
                    .await
                    .map_err(|_| Errno::PubsubSubscribeError)?;

                let message_handler = TopicMessageHandler::new(
                    &topic,
                    match kind {
                        SubscriptionKind::Default => topic_message_handler,
                        SubscriptionKind::DocSync => doc_sync_topic_message_handler,
                    },
                    module_ids,
                );

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
                    Some(peer_id) => Some(
                        hermes_ipfs::PeerId::from_str(&peer_id)
                            .map_err(|_| Errno::InvalidPeerId)?,
                    ),
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
    // Try to stop the node - only works if this is the last reference
    match Arc::try_unwrap(hermes_node) {
        Ok(node) => node.stop().await,
        Err(_) => tracing::warn!("Could not stop IPFS node - other references still exist"),
    }
    Ok(())
}

/// A handler for messages from the IPFS pubsub topic
pub(super) struct TopicMessageHandler<T>
where
    T: Fn(hermes_ipfs::rust_ipfs::GossipsubMessage, String, Option<Vec<ModuleId>>)
        + Send
        + Sync
        + 'static,
{
    /// The topic.
    topic: String,

    /// The handler implementation.
    callback: T,

    /// Module IDs
    module_ids: Option<Vec<ModuleId>>,
}

impl<T> TopicMessageHandler<T>
where
    T: Fn(hermes_ipfs::rust_ipfs::GossipsubMessage, String, Option<Vec<ModuleId>>)
        + Send
        + Sync
        + 'static,
{
    /// Creates the new handler.
    pub fn new(
        topic: &impl ToString,
        callback: T,
        module_ids: Option<Vec<ModuleId>>,
    ) -> Self {
        Self {
            topic: topic.to_string(),
            callback,
            module_ids,
        }
    }

    /// Forwards the message to the handler.
    pub fn handle(
        &self,
        msg: hermes_ipfs::rust_ipfs::GossipsubMessage,
    ) {
        (self.callback)(msg, self.topic.clone(), self.module_ids.clone())
    }
}

/// A handler for subscribe/unsubscribe events from the IPFS pubsub topic
pub(super) struct TopicSubscriptionStatusHandler<T>
where
    T: Fn(hermes_ipfs::SubscriptionStatusEvent, String) + Send + Sync + 'static,
{
    /// The topic.
    topic: String,

    /// The handler implementation.
    callback: T,
}

impl<T> TopicSubscriptionStatusHandler<T>
where
    T: Fn(hermes_ipfs::SubscriptionStatusEvent, String) + Send + Sync + 'static,
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
    module_ids: Option<Vec<ModuleId>>,
) {
    if let Some(ipfs) = HERMES_IPFS.get() {
        let app_names = ipfs.apps.subscribed_apps(SubscriptionKind::Default, &topic);

        drop(
            OnTopicEvent::new(PubsubMessage {
                topic: topic.clone(),
                message: message.data.into(),
                publisher: message.source.map(|p| p.to_string()),
            })
            .build_and_send(app_names, module_ids),
        );
    } else {
        tracing::error!("Failed to send on_topic_event. IPFS is uninitialized");
    }
}

// TODO - Generalize this function, it should be able to handle multiple topics, not just
// .new
/// Handler for Doc Sync `PubSub` messages on "*.new" topics.
///
/// Receives P2P messages containing CBOR-encoded CID lists, spawns an async task
/// to fetch document content from IPFS, and dispatches `OnNewDocEvent` to subscribed
/// apps.
///
/// Uses async file operations (`file_get_async`) to avoid blocking the `PubSub` handler.
/// Message format: `payload::New` → `DocumentDisseminationBody::Docs { docs: Vec<Cid> }`
#[allow(
    clippy::needless_pass_by_value,
    clippy::too_many_lines,
    reason = "The event will be eventually consumed in the handler"
)]
fn doc_sync_topic_message_handler(
    message: hermes_ipfs::rust_ipfs::GossipsubMessage,
    topic: String,
    module_ids: Option<Vec<ModuleId>>,
) {
    if let Ok(msg_str) = std::str::from_utf8(&message.data) {
        tracing::info!("RECEIVED PubSub message on topic: {topic} - data: {msg_str}",);
    }

    // TODO: match the topic against a static list.
    let Some(channel_name) = topic.strip_suffix(".new") else {
        tracing::error!("Handling an IPFS message on a wrong channel.");
        return;
    };

    let payload = match minicbor::decode::<payload::New>(&message.data) {
        Ok(payload) => DocumentDisseminationBody::from(payload),
        Err(err) => {
            tracing::error!(%channel_name, %err, "Failed to decode .new payload from IPFS message");
            return;
        },
    };

    let new_cids = match payload {
        DocumentDisseminationBody::Docs { docs, .. } => docs,
        DocumentDisseminationBody::Manifest { .. } => {
            tracing::error!("Manifest is not supported in a .new payload");
            return;
        },
    };

    // DO NOT remove this log, since it is used in test `_test-pubsub-execute`
    tracing::info!(
        "RECEIVED PubSub message with CIDs: {:?}",
        new_cids
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
    );

    // Use message.source (the publisher) as the provider - this is more reliable than DHT
    // lookup because the publisher just published the content and DHT records may not have
    // propagated yet.
    let publisher = message.source;

    let channel_name_owned = channel_name.to_string();
    let topic_owned = topic.clone();
    // Spawn async task to avoid blocking PubSub handler during file operations
    tokio::spawn(async move {
        tracing::debug!("Inside spawned task, processing {} CIDs", new_cids.len());
        let Some(ipfs) = HERMES_IPFS.get() else {
            tracing::error!("IPFS global instance is uninitialized");
            return;
        };

        let mut contents = Vec::with_capacity(new_cids.len());
        for cid in new_cids {
            tracing::info!("Processing CID: {}", cid.to_string());

            // Use the message publisher as the provider (they just published the content)
            // This avoids DHT lookup latency and propagation issues
            let providers: Vec<hermes_ipfs::PeerId> = match publisher {
                Some(peer_id) => {
                    tracing::info!(%cid, %peer_id, "Using message publisher as provider");
                    vec![peer_id]
                },
                None => {
                    tracing::warn!(%cid, "No message source, falling back to DHT lookup");
                    // Fallback to DHT if message source is unavailable
                    const DHT_PROVIDER_TIMEOUT: Duration = Duration::from_secs(10);
                    let dht_key = cid.to_bytes();
                    match timeout(DHT_PROVIDER_TIMEOUT, ipfs.dht_get_providers_async(dht_key)).await
                    {
                        Ok(Ok(provider_set)) => {
                            let providers: Vec<_> = provider_set.into_iter().collect();
                            tracing::info!(%cid, provider_count = providers.len(), "Found DHT providers");
                            providers
                        },
                        Ok(Err(err)) => {
                            tracing::warn!(%cid, %err, "DHT provider lookup failed");
                            Vec::new()
                        },
                        Err(_) => {
                            tracing::warn!(%cid, "DHT provider lookup timed out");
                            Vec::new()
                        },
                    }
                },
            };

            if providers.is_empty() {
                tracing::error!(%channel_name_owned, %cid, "No providers found, skipping");
                continue;
            }

            // Fetch content with providers (hermes-ipfs connects to them first)
            // Use shorter timeout with retries to handle concurrent request contention
            const MAX_RETRIES: u32 = 5;
            const FETCH_TIMEOUT: Duration = Duration::from_secs(10);
            const BACKOFF_MS: u64 = 200;
            let mut content_result = None;

            for attempt in 0..MAX_RETRIES {
                if attempt > 0 {
                    let backoff_ms = BACKOFF_MS * u64::from(attempt);
                    tracing::info!(%cid, attempt, backoff_ms, "Retrying content fetch after backoff");
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                }

                match timeout(
                    FETCH_TIMEOUT,
                    ipfs.file_get_async_with_providers(&cid, providers.clone()),
                )
                .await
                {
                    Ok(Ok(ipfs_file)) => {
                        content_result = Some(ipfs_file);
                        break;
                    },
                    Ok(Err(err)) => {
                        tracing::warn!(%channel_name_owned, %cid, %err, attempt, "Failed to get content");
                    },
                    Err(_) => {
                        tracing::warn!(%channel_name_owned, %cid, attempt, "Timeout fetching content ({}s)", FETCH_TIMEOUT.as_secs());
                    },
                }
            }

            let content = match content_result {
                Some(c) => c,
                None => {
                    tracing::error!(%channel_name_owned, %cid, "Failed to get content after {} retries", MAX_RETRIES);
                    continue;
                },
            };

            if let Ok(content_str) = std::str::from_utf8(&content) {
                tracing::info!("RECEIVED PubSub message content: {content_str}");
            }

            contents.push(content);
        }
        let app_names = ipfs
            .apps
            .subscribed_apps(SubscriptionKind::DocSync, &topic_owned);

        for content in contents {
            drop(
                OnNewDocEvent::new(&channel_name_owned, &content)
                    .build_and_send(app_names.clone(), module_ids.clone()),
            );
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
