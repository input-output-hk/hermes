//! Hermes IPFS State API
use std::sync::{Arc, Mutex};

use catalyst_types::smt::Tree;

use super::{HERMES_IPFS, SubscriptionKind, is_valid_dht_content, is_valid_pubsub_content};
use crate::{
    app::ApplicationName,
    runtime_extensions::{
        bindings::hermes::ipfs::api::{
            DhtKey, DhtValue, Errno, IpfsContent, IpfsFile, IpfsPath, MessageData, PeerId,
            PubsubTopic,
        },
        hermes::doc_sync,
    },
    wasm::module::ModuleId,
};

/// Add File to IPFS
pub(crate) fn hermes_ipfs_add_file(
    app_name: &ApplicationName,
    contents: IpfsFile,
) -> Result<hermes_ipfs::IpfsPath, Errno> {
    tracing::debug!(app_name = %app_name, "adding IPFS file");
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;
    let ipfs_path = ipfs.file_add(contents)?;
    let ipfs_path_str = ipfs_path.to_string();
    tracing::debug!(app_name = %app_name, path = %ipfs_path_str, "added IPFS file");
    ipfs.apps.pinned_file(app_name.clone(), &ipfs_path_str)?;
    Ok(ipfs_path)
}

/// Validate IPFS Content from DHT or `PubSub`
pub(crate) fn hermes_ipfs_content_validate(
    app_name: &ApplicationName,
    content: &IpfsContent,
) -> bool {
    match content {
        IpfsContent::Dht((k, v)) => {
            let key_str = format!("{k:x?}");
            let is_valid = is_valid_dht_content(k, v);
            tracing::debug!(app_name = %app_name, dht_key = %key_str, is_valid = %is_valid, "DHT value validation");
            is_valid
        },
        IpfsContent::Pubsub((topic, message)) => {
            let is_valid = is_valid_pubsub_content(topic, message);
            tracing::debug!(app_name = %app_name, topic = %topic, is_valid = %is_valid, "PubSub message validation");
            is_valid
        },
    }
}

/// Get File from Ipfs
pub(crate) fn hermes_ipfs_get_file(
    app_name: &ApplicationName,
    path: &IpfsPath,
) -> Result<IpfsFile, Errno> {
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;
    let content = ipfs.file_get(path)?;
    tracing::debug!(app_name = %app_name, path = %path, "got IPFS file with content size {:?}", content.len());
    Ok(content)
}

/// Pin IPFS File
pub(crate) fn hermes_ipfs_pin_file(
    app_name: &ApplicationName,
    path: &IpfsPath,
) -> Result<bool, Errno> {
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;
    tracing::debug!(app_name = %app_name, path = %path, "pin IPFS file");
    let status = ipfs.file_pin(path)?;
    tracing::debug!(app_name = %app_name, path = %path, "pinned IPFS file");
    ipfs.apps.pinned_file(app_name.clone(), path)?;
    Ok(status)
}

/// Un-pin IPFS File
pub(crate) fn hermes_ipfs_unpin_file(
    app_name: &ApplicationName,
    path: &IpfsPath,
) -> Result<bool, Errno> {
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;
    tracing::debug!(app_name = %app_name, path = %path, "un-pin IPFS file");
    let status = ipfs.file_unpin(path)?;
    tracing::debug!(app_name = %app_name, path = %path, "un-pinned IPFS file");
    ipfs.apps.unpinned_file(app_name, path)?;
    Ok(status)
}

/// Get DHT Value
pub(crate) fn hermes_ipfs_get_dht_value(
    app_name: &ApplicationName,
    key: DhtKey,
) -> Result<DhtValue, Errno> {
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;
    let key_str = format!("{key:x?}");
    tracing::debug!(app_name = %app_name, dht_key = %key_str, "get DHT value");
    let value = ipfs.dht_get(key)?;
    tracing::debug!(app_name = %app_name, dht_key = %key_str, "got DHT value");
    Ok(value)
}

/// Put DHT Value
pub(crate) fn hermes_ipfs_put_dht_value(
    app_name: &ApplicationName,
    key: DhtKey,
    value: DhtValue,
) -> Result<bool, Errno> {
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;
    let key_str = format!("{key:x?}");
    tracing::debug!(app_name = %app_name, dht_key = %key_str, "putting DHT value");
    let status = ipfs.dht_put(key.clone(), value)?;
    tracing::debug!(app_name = %app_name, dht_key = %key_str, "have put DHT value");
    ipfs.apps.added_dht_key(app_name.clone(), key);
    Ok(status)
}

/// Marks a node as a provider for the content under the given key.
pub(crate) fn hermes_ipfs_dht_provide(
    app_name: &ApplicationName,
    key: DhtKey,
) -> Result<(), Errno> {
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;
    let key_str = format!("{key:x?}");
    tracing::debug!(app_name = %app_name, dht_key = %key_str, "DHT provide");
    ipfs.dht_provide(key)?;
    tracing::debug!(app_name = %app_name, dht_key = %key_str, "DHT provided");
    Ok(())
}

/// Gets providers of the content under the given key.
pub(crate) fn hermes_ipfs_dht_get_providers(
    app_name: &ApplicationName,
    key: DhtKey,
) -> Result<Vec<PeerId>, Errno> {
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;
    let key_str = format!("{key:x?}");
    tracing::debug!(app_name = %app_name, dht_key = %key_str, "get DHT providers");
    let providers = ipfs.dht_get_providers(key)?;
    tracing::debug!(app_name = %app_name, dht_key = %key_str, "got DHT providers");

    let providers = providers.iter().map(ToString::to_string).collect();
    Ok(providers)
}

/// Returns the peer id of the node.
pub(crate) async fn hermes_ipfs_get_peer_identity(
    peer: Option<PeerId>
) -> Result<Option<hermes_ipfs::PeerInfo>, Errno> {
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;

    let identity = ipfs.get_peer_identity(peer).await?;
    tracing::debug!("Got peer identity");

    Ok(identity)
}

/// Subscribe to a topic.
// TODO[rafal-ch]: Handling of the `ipfs.apps` collection should be extracted to a common
// behaviour for both this and the _blocking version of the function
pub(crate) async fn hermes_ipfs_subscribe(
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
        let handle = ipfs
            .pubsub_subscribe(kind, topic, tree, app_name, module_ids)
            .await?;
        ipfs.apps.added_topic_stream(kind, topic.clone(), handle);
        tracing::debug!(app_name = %app_name, pubsub_topic = %topic, "added subscription topic stream");
    }
    ipfs.apps
        .added_app_topic_subscription(kind, app_name.clone(), topic.clone());
    Ok(true)
}

/// Unsubscribe from a topic
pub(crate) async fn hermes_ipfs_unsubscribe(
    kind: SubscriptionKind,
    app_name: &ApplicationName,
    topic: &PubsubTopic,
) -> Result<bool, Errno> {
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;
    tracing::debug!(app_name = %app_name, pubsub_topic = %topic, "unsubscribing from PubSub topic");

    if ipfs.apps.topic_subscriptions_contains(kind, topic) {
        ipfs.pubsub_unsubscribe(&topic).await?;
        ipfs.apps.removed_topic_stream(kind, topic);
        tracing::debug!(app_name = %app_name, pubsub_topic = %topic, "removed subscription topic stream");
    } else {
        tracing::debug!(app_name = %app_name, pubsub_topic = %topic, "topic subscription does not exist");
    }
    ipfs.apps
        .removed_app_topic_subscription(kind, app_name, topic);
    Ok(true)
}

/// Publish message to a topic
pub(crate) async fn hermes_ipfs_publish(
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

    let res = ipfs.pubsub_publish(topic, message).await;

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

/// Evict Peer from node
pub(crate) fn hermes_ipfs_evict_peer(
    app_name: &ApplicationName,
    peer: PeerId,
) -> Result<bool, Errno> {
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;
    tracing::debug!(app_name = %app_name, peer_id = %peer, "evicting peer");
    let status = ipfs.peer_evict(&peer.to_string())?;
    tracing::debug!(app_name = %app_name, peer_id = %peer, "evicted peer");
    ipfs.apps.evicted_peer(app_name.clone(), peer);
    Ok(status)
}

#[allow(dead_code)]
/// List pinned files
pub(crate) fn hermes_ipfs_ls(app_name: &ApplicationName) -> Result<Vec<String>, Errno> {
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;
    Ok(ipfs.apps.list_pinned_files(app_name))
}
