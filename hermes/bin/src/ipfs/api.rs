//! Hermes IPFS State API
use super::{HERMES_IPFS, is_valid_dht_content, is_valid_pubsub_content};
use crate::{
    app::ApplicationName,
    runtime_extensions::bindings::hermes::ipfs::api::{
        DhtKey, DhtValue, Errno, IpfsContent, IpfsFile, IpfsPath, MessageData, PeerId, PubsubTopic,
    },
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
    tracing::debug!(app_name = %app_name, path = %path, "get IPFS file");
    let content = ipfs.file_get(path)?;
    tracing::debug!(app_name = %app_name, path = %path, "got IPFS file");
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

/// Provide DHT Value
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

/// Subscribe to a topic
pub(crate) fn hermes_ipfs_subscribe(
    app_name: &ApplicationName,
    topic: PubsubTopic,
) -> Result<bool, Errno> {
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;
    tracing::debug!(app_name = %app_name, pubsub_topic = %topic, "subscribing to PubSub topic");
    if ipfs.apps.topic_subscriptions_contains(&topic) {
        tracing::debug!(app_name = %app_name, pubsub_topic = %topic, "topic subscription stream already exists");
    } else {
        let handle = ipfs.pubsub_subscribe(&topic)?;
        ipfs.apps.added_topic_stream(topic.clone(), handle);
        tracing::info!(app_name = %app_name, pubsub_topic = %topic, "added subscription topic stream");
    }
    ipfs.apps
        .added_app_topic_subscription(app_name.clone(), topic);
    Ok(true)
}

/// Publish message to a topic
pub(crate) fn hermes_ipfs_publish(
    _app_name: &ApplicationName,
    topic: &PubsubTopic,
    message: MessageData,
) -> Result<(), Errno> {
    let ipfs = HERMES_IPFS.get().ok_or(Errno::ServiceUnavailable)?;
    ipfs.pubsub_publish(topic.to_string(), message)
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
