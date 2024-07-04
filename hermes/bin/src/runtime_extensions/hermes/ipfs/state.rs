//! Hermes IPFS Internal State

use std::{collections::HashSet, str::FromStr};

use dashmap::{DashMap, DashSet};
use hermes_ipfs::{
    libp2p::futures::{pin_mut, StreamExt},
    AddIpfsFile, Cid, HermesIpfs, IpfsPath as PathIpfsFile, PeerId as TargetPeerId,
};
use once_cell::sync::Lazy;
use tokio::{
    runtime::Builder,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

use crate::{
    app::HermesAppName,
    event::{queue::send, HermesEvent},
    runtime_extensions::{
        bindings::hermes::ipfs::api::{
            DhtKey, DhtValue, Errno, IpfsContent, IpfsFile, IpfsPath, PeerId, PubsubMessage,
            PubsubTopic,
        },
        hermes::ipfs::event::OnTopicEvent,
    },
};

/// Hermes IPFS Internal State
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

/// IPFS Command
enum IpfsCommand {
    /// Add a new IPFS file
    AddFile(AddIpfsFile, oneshot::Sender<Result<IpfsPath, Errno>>),
    /// Get a file from IPFS
    GetFile(PathIpfsFile, oneshot::Sender<Result<Vec<u8>, Errno>>),
    /// Pin a file
    PinFile(Cid, oneshot::Sender<Result<bool, Errno>>),
    /// Get DHT value
    GetDhtValue(DhtKey, oneshot::Sender<Result<DhtValue, Errno>>),
    /// Put DHT value
    PutDhtValue(DhtKey, DhtValue, oneshot::Sender<Result<bool, Errno>>),
    /// Subscribe to a topic
    Subscribe(PubsubTopic, oneshot::Sender<Result<JoinHandle<()>, Errno>>),
    /// Evict Peer from node
    EvictPeer(PeerId, oneshot::Sender<Result<bool, Errno>>),
}

/// A valid DHT Value.
struct ValidDhtValue {}

impl ValidDhtValue {
    /// Checks for `DhtValue` validity.
    fn is_valid(value: &DhtValue) -> bool {
        !value.is_empty()
    }
}
/// A valid `PubsubMessage`
struct ValidPubsubMessage {}

impl ValidPubsubMessage {
    /// Checks for `PubsubMessage` validity.
    fn is_valid(message: &PubsubMessage) -> bool {
        !message.message.is_empty()
    }
}

#[allow(dead_code)]
/// IPFS
async fn ipfs_task(mut queue_rx: mpsc::Receiver<IpfsCommand>) -> anyhow::Result<()> {
    let hermes_node = HermesIpfs::start().await?;
    if let Some(ipfs_command) = queue_rx.recv().await {
        match ipfs_command {
            IpfsCommand::AddFile(ipfs_file, tx) => {
                let ipfs_path = hermes_node.add_ipfs_file(ipfs_file).await?;
                if let Err(_err) = tx.send(Ok(ipfs_path.to_string())) {
                    tracing::error!("Failed to send IPFS path");
                }
            },
            IpfsCommand::GetFile(ipfs_path, tx) => {
                let contents = hermes_node.get_ipfs_file(ipfs_path.into()).await?;
                if let Err(_err) = tx.send(Ok(contents)) {
                    tracing::error!("Failed to get IPFS contents");
                }
            },
            IpfsCommand::PinFile(cid, tx) => {
                let status = match hermes_node.insert_pin(&cid).await {
                    Ok(()) => true,
                    Err(err) => {
                        tracing::error!("Failed to pin block {}: {}", cid, err);
                        false
                    },
                };
                if let Err(err) = tx.send(Ok(status)) {
                    tracing::error!("sending response of pin IPFS file should not fail: {err:?}");
                }
            },
            IpfsCommand::GetDhtValue(key, tx) => {
                let response = hermes_node
                    .dht_get(key)
                    .await
                    .map_err(|_| Errno::DhtGetError);
                if let Err(err) = tx.send(response) {
                    tracing::error!("sending DHT value should not fail: {err:?}");
                }
            },
            IpfsCommand::PutDhtValue(key, value, tx) => {
                let status = hermes_node.dht_put(key, value).await.is_ok();
                if let Err(err) = tx.send(Ok(status)) {
                    tracing::error!("sending status of DHT put should not fail: {err:?}");
                }
            },
            IpfsCommand::Subscribe(topic, tx) => {
                let stream = hermes_node
                    .pubsub_subscribe(topic)
                    .await
                    .map_err(|_| Errno::PubsubSubscribeError)?;
                let handle = tokio::spawn(async move {
                    pin_mut!(stream);
                    while let Some(msg) = stream.next().await {
                        let msg_topic = msg.topic.into_string();
                        let on_topic_event = OnTopicEvent {
                            message: PubsubMessage {
                                topic: msg_topic.clone(),
                                message: String::from_utf8_lossy(&msg.data).to_string(),
                                peer: msg.source.map(|p| p.to_string()),
                            },
                        };
                        let app_names = HERMES_IPFS_STATE.apps.subscribed_apps(&msg_topic);
                        if let Err(err) = send(HermesEvent::new(
                            on_topic_event.clone(),
                            crate::event::TargetApp::List(app_names),
                            crate::event::TargetModule::All,
                        )) {
                            tracing::error!(on_topic_event = ?on_topic_event, "failed to send on_topic_event {err:?}");
                        }
                    }
                });
                if let Err(_err) = tx.send(Ok(handle)) {
                    tracing::error!("Failed to subscribe to topic");
                }
            },
            IpfsCommand::EvictPeer(peer, tx) => {
                let peer_id = TargetPeerId::from_str(&peer).map_err(|_| Errno::InvalidPeerId)?;
                let status = hermes_node.ban_peer(peer_id).await.is_ok();
                if let Err(err) = tx.send(Ok(status)) {
                    tracing::error!("sending status of peer eviction should not fail: {err:?}");
                }
            },
        }
    }
    hermes_node.stop().await;
    Ok(())
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
        IpfsContent::Dht((key, v)) => {
            // TODO(@saibatizoku): Implement types and validation
            let key_str = format!("{key:x?}");
            let is_valid = ValidDhtValue::is_valid(v);
            tracing::debug!(app_name = %app_name, dht_key = %key_str, is_valid = %is_valid, "DHT value validation");
            is_valid
        },
        IpfsContent::Pubsub(m) => {
            // TODO(@saibatizoku): Implement types and validation
            let is_valid = ValidPubsubMessage::is_valid(m);
            tracing::debug!(app_name = %app_name, topic = %m.topic, is_valid = %is_valid, "PubSub message validation");
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
