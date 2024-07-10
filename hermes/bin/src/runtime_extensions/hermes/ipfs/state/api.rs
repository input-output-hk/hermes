//! Hermes IPFS State API
use super::{is_valid_dht_content, is_valid_pubsub_content, HERMES_IPFS_STATE};
use crate::{
    app::HermesAppName,
    runtime_extensions::bindings::hermes::ipfs::api::{
        DhtKey, DhtValue, Errno, IpfsContent, IpfsFile, IpfsPath, MessageData, MessageId, PeerId,
        PubsubTopic,
    },
};

/// Add File to IPFS
pub(crate) fn hermes_ipfs_add_file(
    app_name: &HermesAppName, contents: IpfsFile,
) -> Result<IpfsPath, Errno> {
    tracing::debug!(app_name = %app_name, "adding IPFS file");
    let ipfs_path = HERMES_IPFS_STATE.file_add(contents)?;
    tracing::debug!(app_name = %app_name, path = %ipfs_path, "added IPFS file");
    HERMES_IPFS_STATE
        .apps
        .added_file(app_name.clone(), ipfs_path.clone());
    Ok(ipfs_path)
}

/// Validate IPFS Content from DHT or `PubSub`
pub(crate) fn hermes_ipfs_content_validate(
    app_name: &HermesAppName, content: &IpfsContent,
) -> bool {
    match content {
        IpfsContent::Dht((k, v)) => {
            // TODO(@saibatizoku): Implement types and validation
            let key_str = format!("{k:x?}");
            let is_valid = is_valid_dht_content(k, v);
            tracing::debug!(app_name = %app_name, dht_key = %key_str, is_valid = %is_valid, "DHT value validation");
            is_valid
        },
        IpfsContent::Pubsub((topic, message)) => {
            // TODO(@saibatizoku): Implement types and validation
            let is_valid = is_valid_pubsub_content(topic, message);
            tracing::debug!(app_name = %app_name, topic = %topic, is_valid = %is_valid, "PubSub message validation");
            is_valid
        },
    }
}

/// Get File from Ipfs
pub(crate) fn hermes_ipfs_get_file(
    app_name: &HermesAppName, path: &IpfsPath,
) -> Result<IpfsFile, Errno> {
    tracing::debug!(app_name = %app_name, path = %path, "get IPFS file");
    let content = HERMES_IPFS_STATE.file_get(path)?;
    tracing::debug!(app_name = %app_name, path = %path, "got IPFS file");
    Ok(content)
}

/// Pin IPFS File
pub(crate) fn hermes_ipfs_pin_file(
    app_name: &HermesAppName, path: IpfsPath,
) -> Result<bool, Errno> {
    tracing::debug!(app_name = %app_name, path = %path, "pin IPFS file");
    let status = HERMES_IPFS_STATE.file_pin(&path)?;
    tracing::debug!(app_name = %app_name, path = %path, "pinned IPFS file");
    HERMES_IPFS_STATE.apps.pinned_file(app_name.clone(), path);
    Ok(status)
}

/// Get DHT Value
pub(crate) fn hermes_ipfs_get_dht_value(
    app_name: &HermesAppName, key: DhtKey,
) -> Result<DhtValue, Errno> {
    let key_str = format!("{key:x?}");
    tracing::debug!(app_name = %app_name, dht_key = %key_str, "get DHT value");
    let value = HERMES_IPFS_STATE.dht_get(key)?;
    tracing::debug!(app_name = %app_name, dht_key = %key_str, "got DHT value");
    Ok(value)
}

/// Put DHT Value
pub(crate) fn hermes_ipfs_put_dht_value(
    app_name: &HermesAppName, key: DhtKey, value: DhtValue,
) -> Result<bool, Errno> {
    let key_str = format!("{key:x?}");
    tracing::debug!(app_name = %app_name, dht_key = %key_str, "putting DHT value");
    let status = HERMES_IPFS_STATE.dht_put(key.clone(), value)?;
    tracing::debug!(app_name = %app_name, dht_key = %key_str, "have put DHT value");
    HERMES_IPFS_STATE.apps.added_dht_key(app_name.clone(), key);
    Ok(status)
}

/// Subscribe to a topic
pub(crate) fn hermes_ipfs_subscribe(
    app_name: &HermesAppName, topic: PubsubTopic,
) -> Result<bool, Errno> {
    tracing::debug!(app_name = %app_name, pubsub_topic = %topic, "subscribing to PubSub topic");
    if HERMES_IPFS_STATE.apps.topic_subscriptions_contains(&topic) {
        tracing::debug!(app_name = %app_name, pubsub_topic = %topic, "topic subscription stream already exists");
    } else {
        let handle = HERMES_IPFS_STATE.pubsub_subscribe(&topic)?;
        HERMES_IPFS_STATE
            .apps
            .added_topic_stream(topic.clone(), handle);
        tracing::debug!(app_name = %app_name, pubsub_topic = %topic, "added subscription topic stream");
    }
    HERMES_IPFS_STATE
        .apps
        .added_app_topic_subscription(app_name.clone(), topic);
    Ok(true)
}

/// Publish message to a topic
pub(crate) fn hermes_ipfs_publish(
    _app_name: &HermesAppName, topic: &PubsubTopic, message: MessageData,
) -> Result<MessageId, Errno> {
    let message_id = HERMES_IPFS_STATE.pubsub_publish(topic.to_string(), message)?;
    Ok(message_id.0)
}

/// Evict Peer from node
pub(crate) fn hermes_ipfs_evict_peer(
    app_name: &HermesAppName, peer: PeerId,
) -> Result<bool, Errno> {
    tracing::debug!(app_name = %app_name, peer_id = %peer, "evicting peer");
    let status = HERMES_IPFS_STATE.peer_evict(&peer.to_string())?;
    tracing::debug!(app_name = %app_name, peer_id = %peer, "evicted peer");
    HERMES_IPFS_STATE.apps.evicted_peer(app_name.clone(), peer);
    Ok(status)
}
