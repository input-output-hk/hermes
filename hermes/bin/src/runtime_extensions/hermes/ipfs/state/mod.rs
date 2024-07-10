//! Hermes IPFS Internal State
mod task;

use std::{collections::HashSet, str::FromStr};

use dashmap::{DashMap, DashSet};
use hermes_ipfs::{AddIpfsFile, IpfsPath as PathIpfsFile, PubsubMessageId};
use once_cell::sync::Lazy;
use task::{ipfs_task, IpfsCommand};
use tokio::{
    runtime::Builder,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

use crate::{
    app::HermesAppName,
    runtime_extensions::bindings::hermes::ipfs::api::{
        DhtKey, DhtValue, Errno, IpfsContent, IpfsFile, IpfsPath, MessageData, MessageId, PeerId,
        PubsubTopic,
    },
};

/// Hermes IPFS Internal State
///
/// This is a wrapper around `HermesIpfsState` which provides a singleton instance of the
/// IPFS state.
///
/// This is done to ensure the IPFS state is initialized only once when the
/// `HermesIpfsState` is first used. This is done to avoid any issues that may arise if
/// the IPFS state is initialized multiple times.
///
/// The IPFS state is initialized in a separate thread and the sender channel is stored in
/// the `HermesIpfsState`.
static HERMES_IPFS_STATE: Lazy<HermesIpfsState> = Lazy::new(|| {
    let sender = if let Ok(runtime) = Builder::new_current_thread().enable_all().build() {
        let (sender, receiver) = mpsc::channel(1);
        let _handle = std::thread::spawn(move || {
            runtime.block_on(async move {
                let h = tokio::spawn(ipfs_task(receiver));
                drop(tokio::join!(h));
            });
            std::process::exit(0);
        });
        Some(sender)
    } else {
        // Failed to start the IPFS task
        tracing::error!("Failed to start the IPFS task");
        None
    };
    HermesIpfsState::new(sender)
});

/// Hermes IPFS Internal State
struct HermesIpfsState {
    /// State related to `HermesAppName`
    apps: AppIpfsState,
}

impl HermesIpfsState {
    /// Create a new `HermesIpfsState`
    fn new(sender: Option<mpsc::Sender<IpfsCommand>>) -> Self {
        Self {
            apps: AppIpfsState::new(sender),
        }
    }

    /// Add file
    ///
    /// Returns the IPFS path of the added file
    ///
    /// ## Parameters
    /// - `contents`: The content to add
    ///
    /// ## Errors
    /// - `Errno::FileAddError`: Failed to add the content
    fn file_add(&self, contents: IpfsFile) -> Result<IpfsPath, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.apps
            .sender
            .as_ref()
            .ok_or(Errno::FileAddError)?
            .blocking_send(IpfsCommand::AddFile(
                AddIpfsFile::Stream((None, contents)),
                cmd_tx,
            ))
            .map_err(|_| Errno::FileAddError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::FileAddError)?
    }

    #[allow(clippy::needless_pass_by_value)]
    /// Get file
    ///
    /// Returns the content of the file
    ///
    /// ## Parameters
    /// - `ipfs_path`: The IPFS path of the file
    ///
    /// ## Errors
    /// - `Errno::InvalidIpfsPath`: Invalid IPFS path
    /// - `Errno::FileGetError`: Failed to get the file
    fn file_get(&self, ipfs_path: IpfsPath) -> Result<IpfsFile, Errno> {
        let ipfs_path = PathIpfsFile::from_str(&ipfs_path).map_err(|_| Errno::InvalidIpfsPath)?;
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.apps
            .sender
            .as_ref()
            .ok_or(Errno::FileGetError)?
            .blocking_send(IpfsCommand::GetFile(ipfs_path, cmd_tx))
            .map_err(|_| Errno::FileGetError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::FileGetError)?
    }

    #[allow(clippy::needless_pass_by_value)]
    /// Pin file
    ///
    /// ## Parameters
    /// - `ipfs_path`: The IPFS path of the file
    ///
    /// ## Errors
    /// - `Errno::InvalidCid`: Invalid CID
    /// - `Errno::InvalidIpfsPath`: Invalid IPFS path
    /// - `Errno::FilePinError`: Failed to pin the file
    fn file_pin(&self, ipfs_path: IpfsPath) -> Result<bool, Errno> {
        let ipfs_path = PathIpfsFile::from_str(&ipfs_path).map_err(|_| Errno::InvalidIpfsPath)?;
        let cid = ipfs_path.root().cid().ok_or(Errno::InvalidCid)?;
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.apps
            .sender
            .as_ref()
            .ok_or(Errno::FilePinError)?
            .blocking_send(IpfsCommand::PinFile(*cid, cmd_tx))
            .map_err(|_| Errno::FilePinError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::FilePinError)?
    }

    /// Put DHT Key-Value
    fn dht_put(&self, key: DhtKey, value: DhtValue) -> Result<bool, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.apps
            .sender
            .as_ref()
            .ok_or(Errno::DhtPutError)?
            .blocking_send(IpfsCommand::PutDhtValue(key, value, cmd_tx))
            .map_err(|_| Errno::DhtPutError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::DhtPutError)?
    }

    /// Get DHT Value by Key
    fn dht_get(&self, key: DhtKey) -> Result<DhtValue, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.apps
            .sender
            .as_ref()
            .ok_or(Errno::DhtGetError)?
            .blocking_send(IpfsCommand::GetDhtValue(key, cmd_tx))
            .map_err(|_| Errno::DhtGetError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::DhtGetError)?
    }

    /// Publish message to a `PubSub` topic
    fn pubsub_publish(
        &self, topic: PubsubTopic, message: MessageData,
    ) -> Result<PubsubMessageId, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.apps
            .sender
            .as_ref()
            .ok_or(Errno::PubsubPublishError)?
            .blocking_send(IpfsCommand::Publish(topic, message, cmd_tx))
            .map_err(|_| Errno::PubsubPublishError)?;
        cmd_rx
            .blocking_recv()
            .map_err(|_| Errno::PubsubPublishError)?
    }

    #[allow(clippy::needless_pass_by_value)]
    /// Subscribe to a `PubSub` topic
    fn pubsub_subscribe(&self, topic: PubsubTopic) -> Result<JoinHandle<()>, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.apps
            .sender
            .as_ref()
            .ok_or(Errno::PubsubSubscribeError)?
            .blocking_send(IpfsCommand::Subscribe(topic, cmd_tx))
            .map_err(|_| Errno::PubsubSubscribeError)?;
        cmd_rx
            .blocking_recv()
            .map_err(|_| Errno::PubsubSubscribeError)?
    }

    #[allow(clippy::needless_pass_by_value)]
    /// Evict peer
    fn peer_evict(&self, peer: PeerId) -> Result<bool, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.apps
            .sender
            .as_ref()
            .ok_or(Errno::PeerEvictionError)?
            .blocking_send(IpfsCommand::EvictPeer(peer, cmd_tx))
            .map_err(|_| Errno::PeerEvictionError)?;
        cmd_rx
            .blocking_recv()
            .map_err(|_| Errno::PeerEvictionError)?
    }
}

/// IPFS app state
struct AppIpfsState {
    /// Send events to the IPFS node.
    sender: Option<mpsc::Sender<IpfsCommand>>,
    /// List of uploaded files per app.
    published_files: DashMap<HermesAppName, DashSet<IpfsPath>>,
    /// List of pinned files per app.
    pinned_files: DashMap<HermesAppName, DashSet<IpfsPath>>,
    /// List of DHT values per app.
    dht_keys: DashMap<HermesAppName, DashSet<DhtKey>>,
    /// List of subscriptions per app.
    topic_subscriptions: DashMap<PubsubTopic, HashSet<HermesAppName>>,
    /// Collection of stream join handles per topic subscription.
    subscriptions_streams: DashMap<PubsubTopic, JoinHandle<()>>,
    /// List of evicted peers per app.
    evicted_peers: DashMap<HermesAppName, DashSet<PeerId>>,
}

impl AppIpfsState {
    /// Create new `AppIpfsState`
    fn new(sender: Option<mpsc::Sender<IpfsCommand>>) -> Self {
        Self {
            sender,
            published_files: DashMap::default(),
            pinned_files: DashMap::default(),
            dht_keys: DashMap::default(),
            topic_subscriptions: DashMap::default(),
            subscriptions_streams: DashMap::default(),
            evicted_peers: DashMap::default(),
        }
    }

    /// Keep track of `ipfs_path` from file added by an app.
    fn added_file(&self, app_name: HermesAppName, ipfs_path: IpfsPath) {
        self.published_files
            .entry(app_name)
            .or_default()
            .value_mut()
            .insert(ipfs_path);
    }

    /// Keep track of `ipfs_path` of file pinned by an app.
    fn pinned_file(&self, app_name: HermesAppName, ipfs_path: IpfsPath) {
        self.pinned_files
            .entry(app_name)
            .or_default()
            .value_mut()
            .insert(ipfs_path);
    }

    /// Keep track of `dht_key` of DHT value added by an app.
    fn added_dht_key(&self, app_name: HermesAppName, dht_key: DhtKey) {
        self.dht_keys
            .entry(app_name)
            .or_default()
            .value_mut()
            .insert(dht_key);
    }

    /// Keep track of `topic` subscription added by an app.
    fn added_app_topic_subscription(&self, app_name: HermesAppName, topic: PubsubTopic) {
        self.topic_subscriptions
            .entry(topic)
            .or_default()
            .value_mut()
            .insert(app_name);
    }

    /// Keep track of `topic` stream handle.
    fn added_topic_stream(&self, topic: PubsubTopic, handle: JoinHandle<()>) {
        self.subscriptions_streams.entry(topic).insert(handle);
    }

    /// Check if a topic subscription already exists.
    fn topic_subscriptions_contains(&self, topic: &PubsubTopic) -> bool {
        self.topic_subscriptions.contains_key(topic)
    }

    /// Returns a list of apps subscribed to a topic.
    fn subscribed_apps(&self, topic: &PubsubTopic) -> Vec<HermesAppName> {
        self.topic_subscriptions
            .get(topic)
            .map_or(vec![], |apps| apps.value().iter().cloned().collect())
    }

    /// Add `peer_id` of evicted peer by an app.
    fn evicted_peer(&self, app_name: HermesAppName, peer_id: PeerId) {
        self.evicted_peers
            .entry(app_name)
            .or_default()
            .value_mut()
            .insert(peer_id);
    }
}

/// Checks for `DhtKey`, and `DhtValue` validity.
fn is_valid_dht_content(_key: &DhtKey, value: &DhtValue) -> bool {
    // TODO(@anyone): Implement DHT content validation
    !value.is_empty()
}

/// Checks for `PubsubTopic`, and `MessageData` validity.
fn is_valid_pubsub_content(_topic: &PubsubTopic, message: &MessageData) -> bool {
    // TODO(@anyone): Implement PubSub content validation
    !message.is_empty()
}

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
    let content = HERMES_IPFS_STATE.file_get(path.to_string())?;
    tracing::debug!(app_name = %app_name, path = %path, "got IPFS file");
    Ok(content)
}

/// Pin IPFS File
pub(crate) fn hermes_ipfs_pin_file(
    app_name: &HermesAppName, path: IpfsPath,
) -> Result<bool, Errno> {
    tracing::debug!(app_name = %app_name, path = %path, "pin IPFS file");
    let status = HERMES_IPFS_STATE.file_pin(path.clone())?;
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
        let handle = HERMES_IPFS_STATE.pubsub_subscribe(topic.to_string())?;
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
    let status = HERMES_IPFS_STATE.peer_evict(peer.to_string())?;
    tracing::debug!(app_name = %app_name, peer_id = %peer, "evicted peer");
    HERMES_IPFS_STATE.apps.evicted_peer(app_name.clone(), peer);
    Ok(status)
}
