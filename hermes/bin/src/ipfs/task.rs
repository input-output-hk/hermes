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
    while let Some(ipfs_command) = queue_rx.recv().await {
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
                let response = match hermes_node.insert_pin(&cid).await {
                    Ok(()) => Ok(true),
                    Err(err) if err.to_string().contains("already pinned recursively") => {
                        tracing::debug!(cid = %cid, "file already pinned");
                        Ok(true)
                    },
                    Err(err) => {
                        tracing::error!(cid = %cid, "failed to pin: {}", err);
                        Ok(false)
                    },
                };
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
                dbg!();
                let stream = hermes_node
                    .pubsub_subscribe(&topic)
                    .await
                    .map_err(|_| Errno::PubsubSubscribeError)?;
                dbg!();

                let message_handler = TopicMessageHandler::new(&topic, match kind {
                    SubscriptionKind::Default => topic_message_handler,
                    SubscriptionKind::DocSync => doc_sync_topic_message_handler,
                });
                dbg!();

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

/// Handler function for topic message streams (Doc Sync).
#[allow(
    clippy::needless_pass_by_value,
    reason = "The event will be eventually consumed in the handler"
)]
fn doc_sync_topic_message_handler(
    message: hermes_ipfs::rust_ipfs::GossipsubMessage,
    topic: String,
) {
    let Some(ipfs) = HERMES_IPFS.get() else {
        tracing::error!("Failed to send on_new_doc event. IPFS is uninitialized");
        return;
    };

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

    let mut contents = Vec::with_capacity(new_cids.len());

    for cid in new_cids {
        let path = hermes_ipfs::IpfsPath::new(PathRoot::Ipld(cid)).to_string();

        // Not tracking any app that pinned it (via `ipfs::api::hermes_ipfs_pin_file`),
        // since the pin is re-used by all apps.
        match ipfs.file_pin(&path) {
            Err(err) => {
                tracing::error!(%channel_name, %cid, %err, "Failed to pin a document");
                continue;
            },
            Ok(false) => {
                tracing::error!(%channel_name, %cid, "Failed to pin a document (unspecified error)");
                continue;
            },
            Ok(true) => (),
        }

        let content = match ipfs.file_get(&path) {
            Ok(ipfs_file) => ipfs_file,
            Err(err) => {
                tracing::error!(
                    %channel_name, %cid, %err,
                    "Failed to get content of the document after a successful IPFS pin"
                );
                continue;
            },
        };

        contents.push(content);
    }

    let app_names = ipfs.apps.subscribed_apps(SubscriptionKind::DocSync, &topic);

    for content in contents {
        let event = HermesEvent::new(
            OnNewDocEvent::new(channel_name, &content),
            crate::event::TargetApp::List(app_names.clone()),
            crate::event::TargetModule::All,
        );

        // Dispatch Hermes Event
        if let Err(err) = send(event) {
            tracing::error!(%channel_name, %err, "Failed to send `on_new_doc` event");
        }
    }
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
