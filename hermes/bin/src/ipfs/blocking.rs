use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

use catalyst_types::smt::Tree;
use tokio::{sync::oneshot, task::JoinHandle};

use super::task::IpfsCommand;
pub(crate) use super::task::SubscriptionKind;
use crate::{
    app::ApplicationName,
    ipfs::{HERMES_IPFS, HermesIpfsNode},
    runtime_extensions::{
        bindings::hermes::ipfs::api::{Errno, MessageData, PeerId, PubsubTopic},
        hermes,
        hermes::doc_sync,
    },
    wasm::module::ModuleId,
};

impl<N> HermesIpfsNode<N>
where N: hermes_ipfs::rust_ipfs::NetworkBehaviour<ToSwarm = Infallible> + Send + Sync
{
    /// Get the peer identity in a non-async context.
    pub(super) fn get_peer_identity_blocking(
        &self,
        peer: Option<PeerId>,
    ) -> Result<Option<hermes_ipfs::PeerInfo>, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::GetPeerIdError)?
            .blocking_send(IpfsCommand::Identity(peer, cmd_tx))
            .map_err(|_| Errno::GetPeerIdError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::GetPeerIdError)?
    }

    /// Publish message to a `PubSub` topic in the non-async context.
    pub(super) fn pubsub_publish_blocking(
        &self,
        topic: &PubsubTopic,
        message: MessageData,
    ) -> Result<(), Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::PubsubPublishError)?
            .blocking_send(IpfsCommand::Publish(topic.clone(), message, cmd_tx))
            .map_err(|_| Errno::PubsubPublishError)?;
        cmd_rx
            .blocking_recv()
            .map_err(|_| Errno::PubsubPublishError)?
    }

    /// Subscribe to a `PubSub` topic in a non-async context.
    pub(super) fn pubsub_subscribe_blocking(
        &self,
        kind: SubscriptionKind,
        topic: &PubsubTopic,
        tree: Option<Arc<Mutex<Tree<hermes::doc_sync::Cid>>>>,
        app_name: &ApplicationName,
        module_ids: Option<&Vec<ModuleId>>,
    ) -> Result<JoinHandle<()>, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        let module_ids_owned = module_ids.cloned();
        self.sender
            .as_ref()
            .ok_or(Errno::PubsubSubscribeError)?
            .blocking_send(IpfsCommand::Subscribe(
                topic.clone(),
                kind,
                tree,
                app_name.clone(),
                module_ids_owned,
                cmd_tx,
            ))
            .map_err(|_| Errno::PubsubSubscribeError)?;
        cmd_rx
            .blocking_recv()
            .map_err(|_| Errno::PubsubSubscribeError)?
    }

    /// Unsubscribe from a `PubSub` topic in the non-async context
    pub(super) fn pubsub_unsubscribe_blocking(
        &self,
        topic: &PubsubTopic,
    ) -> Result<(), Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::PubsubUnsubscribeError)?
            .blocking_send(IpfsCommand::Unsubscribe(topic.clone(), cmd_tx))
            .map_err(|_| Errno::PubsubUnsubscribeError)?;
        cmd_rx
            .blocking_recv()
            .map_err(|_| Errno::PubsubUnsubscribeError)?
    }
}

/// Returns the peer id of the node in the non-async context.
pub(crate) fn hermes_ipfs_get_peer_identity(
    peer: Option<PeerId>
) -> Result<Option<hermes_ipfs::PeerInfo>, Errno> {
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;

    let identity = ipfs.get_peer_identity_blocking(peer)?;
    tracing::debug!("Got peer identity");

    Ok(identity)
}

/// Subscribe to a topic from in the non-async context.
pub(crate) fn hermes_ipfs_subscribe(
    kind: SubscriptionKind,
    app_name: &ApplicationName,
    tree: Option<Arc<Mutex<Tree<doc_sync::Cid>>>>,
    topic: &PubsubTopic,
    module_ids: Option<&Vec<ModuleId>>,
) -> Result<bool, Errno> {
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;
    tracing::debug!(app_name = %app_name, pubsub_topic = %topic, "subscribing to PubSub topic");
    if ipfs.apps.topic_subscriptions_contains(kind, topic) {
        tracing::debug!(app_name = %app_name, pubsub_topic = %topic, "topic subscription stream already exists");
    } else {
        let handle = ipfs.pubsub_subscribe_blocking(kind, topic, tree, app_name, module_ids)?;
        ipfs.apps.added_topic_stream(kind, topic.clone(), handle);
        tracing::debug!(app_name = %app_name, pubsub_topic = %topic, "added subscription topic stream");
    }
    ipfs.apps
        .added_app_topic_subscription(kind, app_name.clone(), topic.clone());
    Ok(true)
}

/// Unsubscribe from a topic in the non-async context
pub(crate) fn hermes_ipfs_unsubscribe(
    kind: SubscriptionKind,
    app_name: &ApplicationName,
    topic: &PubsubTopic,
) -> Result<bool, Errno> {
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;
    tracing::debug!(app_name = %app_name, pubsub_topic = %topic, "unsubscribing from PubSub topic");

    if ipfs.apps.topic_subscriptions_contains(kind, topic) {
        ipfs.pubsub_unsubscribe_blocking(topic)?;

        ipfs.apps.removed_topic_stream(kind, topic);
        tracing::debug!(app_name = %app_name, pubsub_topic = %topic, "removed subscription topic
stream");
    } else {
        tracing::debug!(app_name = %app_name, pubsub_topic = %topic, "topic subscription does not
exist");
    }
    ipfs.apps
        .removed_app_topic_subscription(kind, app_name, topic);
    Ok(true)
}

/// Publish message to a topic in the non-async context.
pub(crate) fn hermes_ipfs_publish(
    app_name: &ApplicationName,
    topic: &PubsubTopic,
    message: MessageData,
) -> Result<(), Errno> {
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;

    // Log publish attempt with message size
    tracing::info!(
    app_name = %app_name,
    topic = %topic,
    message_size = message.len(),
    "📤 Publishing PubSub message"
    );

    let res = ipfs.pubsub_publish_blocking(topic, message);

    match &res {
        Ok(()) => {
            tracing::info!(
            app_name = %app_name,
            topic = %topic,
            "✅ PubSub publish succeeded"
            );
        },
        Err(e) => {
            tracing::error!(
            app_name = %app_name,
            topic = %topic,
            error = ?e,
            "❌ PubSub publish failed"
            );
        },
    }

    res
}
