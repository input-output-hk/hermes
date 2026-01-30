//! IPFS Task
use std::{
    collections::HashSet,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use catalyst_types::smt::Tree;
use hermes_ipfs::{
    Cid, HermesIpfs, IpfsPath as PathIpfsFile, PeerId as TargetPeerId,
    doc_sync::payload::{self},
};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::timeout,
};

use super::HERMES_IPFS;
use crate::{
    app::ApplicationName,
    ipfs::{doc_sync::handle_doc_sync_topic, topic_message_context::TopicMessageContext},
    runtime_extensions::{
        bindings::hermes::ipfs::api::{
            DhtKey, DhtValue, Errno, IpfsFile, MessageData, PeerId, PubsubMessage, PubsubTopic,
        },
        hermes::{
            doc_sync::{self, OnNewDocEvent},
            ipfs::event::OnTopicEvent,
        },
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
        Option<Arc<Mutex<Tree<doc_sync::Cid>>>>,
        ApplicationName,
        Option<Vec<ModuleId>>,
        oneshot::Sender<Result<JoinHandle<()>, Errno>>,
    ),
    /// Unsubscribe from a topic
    Unsubscribe(PubsubTopic, oneshot::Sender<Result<(), Errno>>),
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
            IpfsCommand::Subscribe(topic, kind, tree, app_name, module_ids, tx) => {
                tracing::info!(topic, "received Subscribe request");
                let stream = hermes_node
                    .pubsub_subscribe(&topic)
                    .await
                    .map_err(|_| Errno::PubsubSubscribeError)?;

                let message_handler = match kind {
                    SubscriptionKind::Default => {
                        TopicMessageHandler::new(
                            &topic,
                            topic_message_handler,
                            TopicMessageContext::new(None, app_name, module_ids),
                        )
                    },
                    SubscriptionKind::DocSync => {
                        TopicMessageHandler::new(
                            &topic,
                            doc_sync_topic_message_handler,
                            TopicMessageContext::new(tree, app_name, module_ids),
                        )
                    },
                };

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
            IpfsCommand::Unsubscribe(topic, tx) => {
                tracing::info!(topic, "received Unsubscribe request");
                hermes_node
                    .pubsub_unsubscribe(topic)
                    .await
                    .map_err(|_| Errno::PubsubUnsubscribeError)?;
                send_response(Ok(()), tx);
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
    // Try to stop the node - only works if this is the last reference
    if let Ok(node) = Arc::try_unwrap(hermes_node) {
        node.stop().await;
    } else {
        tracing::warn!("Could not stop IPFS node - other references still exist");
    }
    Ok(())
}

/// A handler for messages from the IPFS pubsub topic
pub(super) struct TopicMessageHandler {
    /// The topic.
    topic: String,

    /// The handler implementation.
    #[allow(
        clippy::type_complexity,
        reason = "to be revisited after the doc sync functionality is fully implemented as this type still evolves"
    )]
    callback: Box<
        dyn Fn(hermes_ipfs::rust_ipfs::GossipsubMessage, String, &TopicMessageContext)
            + Send
            + Sync
            + 'static,
    >,

    /// The context.
    context: TopicMessageContext,
}

impl TopicMessageHandler {
    /// Creates the new handler.
    pub fn new<F>(
        topic: &impl ToString,
        callback: F,
        context: TopicMessageContext,
    ) -> Self
    where
        F: Fn(hermes_ipfs::rust_ipfs::GossipsubMessage, String, &TopicMessageContext)
            + Send
            + Sync
            + 'static,
    {
        Self {
            topic: topic.to_string(),
            callback: Box::new(callback),
            context,
        }
    }

    /// Forwards the message to the handler.
    pub fn handle(
        &self,
        msg: hermes_ipfs::rust_ipfs::GossipsubMessage,
    ) {
        (self.callback)(msg, self.topic.clone(), &self.context);
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
    context: &TopicMessageContext,
) {
    if let Some(ipfs) = HERMES_IPFS.get() {
        let app_names = ipfs.apps.subscribed_apps(SubscriptionKind::Default, &topic);

        drop(
            OnTopicEvent::new(PubsubMessage {
                topic,
                message: message.data.into(),
                publisher: message.source.map(|p| p.to_string()),
            })
            .build_and_send(app_names, context.module_ids()),
        );
    } else {
        tracing::error!("Failed to send on_topic_event. IPFS is uninitialized");
    }
}

/// Handler for Doc Sync `PubSub` messages.
#[allow(
    clippy::needless_pass_by_value,
    reason = "the other handler consumes the message and we need to keep the signatures consistent"
)]
fn doc_sync_topic_message_handler(
    message: hermes_ipfs::rust_ipfs::GossipsubMessage,
    topic: String,
    context: &TopicMessageContext,
) {
    if let Ok(msg_str) = std::str::from_utf8(&message.data) {
        tracing::info!(
            "RECEIVED PubSub message on topic: {topic} - data: {}",
            &msg_str.chars().take(100).collect::<String>()
        );
    }

    let result = handle_doc_sync_topic::<payload::New>(&message, &topic, context);
    if let Some(Err(err)) = result {
        tracing::error!("Failed to handle IPFS message: {}", err);
    }
}

/// Processes the received CIDs from a broadcasted message.
pub(super) fn process_broadcasted_cids(
    topic: &str,
    channel_name: &str,
    cids: Vec<Cid>,
    publisher: Option<hermes_ipfs::PeerId>,
    module_ids: Option<&Vec<ModuleId>>,
) {
    let channel_name_owned = channel_name.to_string();
    let topic_owned = topic.to_string();
    let module_ids_owned = module_ids.cloned();
    // Spawn async task to avoid blocking PubSub handler during file operations
    tokio::spawn(async move {
        // Fetch content with providers (hermes-ipfs connects to them first)
        // Use shorter timeout with retries to handle concurrent request contention
        /// Maximum number of retries
        const MAX_RETRIES: u32 = 5;
        /// How long to wait between retries
        const FETCH_TIMEOUT: Duration = Duration::from_secs(10);
        /// How long to wait between retries
        const BACKOFF_MS: u64 = 200;

        tracing::debug!("Inside spawned task, processing {} CIDs", cids.len());
        let Some(ipfs) = HERMES_IPFS.get() else {
            tracing::error!("IPFS global instance is uninitialized");
            return;
        };

        let mut contents = Vec::with_capacity(cids.len());
        for cid in cids {
            tracing::info!("Processing CID: {}", cid.to_string());

            // Use the message publisher as the provider (they just published the content)
            // This avoids DHT lookup latency and propagation issues
            let providers: Vec<hermes_ipfs::PeerId> = if let Some(peer_id) = publisher {
                tracing::info!(%cid, %peer_id, "Using message publisher as provider");
                vec![peer_id]
            } else {
                /// Fallback to DHT if message source is unavailable
                const DHT_PROVIDER_TIMEOUT: Duration = Duration::from_secs(10);

                tracing::warn!(%cid, "No message source, falling back to DHT lookup");
                let dht_key = cid.to_bytes();
                match timeout(DHT_PROVIDER_TIMEOUT, ipfs.dht_get_providers_async(dht_key)).await {
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
            };

            if providers.is_empty() {
                tracing::error!(%channel_name_owned, %cid, "No providers found, skipping");
                continue;
            }

            let mut content_result = None;

            for attempt in 0..MAX_RETRIES {
                if attempt > 0 {
                    let backoff_ms = BACKOFF_MS.saturating_mul(u64::from(attempt));

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

            let Some(content) = content_result else {
                tracing::error!(%channel_name_owned, %cid, "Failed to get content after {MAX_RETRIES} retries");
                continue;
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
                    .build_and_send(app_names.clone(), module_ids_owned.as_ref()),
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
