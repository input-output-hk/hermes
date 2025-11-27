//! Hermes IPFS service.
mod api;
mod task;

use std::{
    collections::HashSet, convert::Infallible, marker::PhantomData, path::Path, str::FromStr,
};

pub(crate) use api::{
    hermes_ipfs_add_file, hermes_ipfs_content_validate, hermes_ipfs_evict_peer,
    hermes_ipfs_get_dht_value, hermes_ipfs_get_file, hermes_ipfs_pin_file, hermes_ipfs_publish,
    hermes_ipfs_put_dht_value, hermes_ipfs_subscribe, hermes_ipfs_unpin_file,
};
use dashmap::DashMap;
use hermes_ipfs::{
    AddIpfsFile, Cid, HermesIpfs, HermesIpfsBuilder, IpfsPath as BaseIpfsPath, rust_ipfs::dummy,
};
use once_cell::sync::OnceCell;
pub(crate) use task::SubscriptionKind;
use task::{IpfsCommand, ipfs_command_handler};
use tokio::{
    runtime::Builder,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

use crate::{
    app::ApplicationName,
    runtime_extensions::bindings::hermes::ipfs::api::{
        DhtKey, DhtValue, Errno, IpfsFile, IpfsPath, MessageData, PeerId, PubsubTopic,
    },
};

/// Hermes IPFS Internal Node
///
/// This is a wrapper around `HermesIpfsNode` which provides a singleton instance of the
/// IPFS state.
///
/// This is done to ensure the IPFS Node is initialized only once when the
/// `HermesIpfsNode` is first used. This is done to avoid any issues that may arise if
/// the IPFS Node is initialized multiple times.
///
/// The IPFS Node is initialized in a separate thread and the sender channel is stored in
/// the `HermesIpfsNode`.
pub(crate) static HERMES_IPFS: OnceCell<HermesIpfsNode<dummy::Behaviour>> = OnceCell::new();

/// IPFS bootstrap config.
#[derive(Copy, Clone, Debug)]
pub struct Config<'a> {
    /// Local base directory.
    pub base_dir: &'a Path,
    /// Should the default addresses be bound.
    pub default_bootstrap: bool,
}

/// Bootstrap `HERMES_IPFS` node.
///
/// ## Errors
///
/// Returns errors if IPFS node fails to start.
pub fn bootstrap(config: Config) -> anyhow::Result<()> {
    let ipfs_data_path = config.base_dir.join("ipfs");
    if !ipfs_data_path.exists() {
        tracing::info!("creating IPFS repo directory: {}", ipfs_data_path.display());
        std::fs::create_dir_all(&ipfs_data_path)?;
    }
    let ipfs_node = HermesIpfsNode::init(
        HermesIpfsBuilder::new()
            .with_default()
            .set_disk_storage(ipfs_data_path.clone()),
        config.default_bootstrap,
    )?;
    HERMES_IPFS
        .set(ipfs_node)
        .map_err(|_| anyhow::anyhow!("failed to start IPFS node"))?;
    Ok(())
}

/// Hermes IPFS Internal Node
pub(crate) struct HermesIpfsNode<N>
where N: hermes_ipfs::rust_ipfs::NetworkBehaviour<ToSwarm = Infallible> + Send + Sync
{
    /// Send events to the IPFS node.
    sender: Option<mpsc::Sender<IpfsCommand>>,
    /// State related to `ApplicationName`
    apps: AppIpfsState,
    /// Phantom data.
    _phantom_data: PhantomData<N>,
}

impl<N> HermesIpfsNode<N>
where N: hermes_ipfs::rust_ipfs::NetworkBehaviour<ToSwarm = Infallible> + Send + Sync
{
    /// Create, initialize, and bootstrap a new `HermesIpfsNode`
    pub(crate) fn init(
        builder: HermesIpfsBuilder<N>,
        default_bootstrap: bool,
    ) -> anyhow::Result<Self> {
        let runtime = Builder::new_current_thread().enable_all().build()?;
        let (sender, receiver) = mpsc::channel(1);

        // Build and start IPFS node, before moving into the thread
        let node = runtime.block_on(async move { builder.start().await })?;

        let _handle = std::thread::spawn(move || {
            let _unused = runtime.block_on(async move {
                if default_bootstrap {
                    // Add default addresses for bootstrapping
                    let addresses = node.default_bootstrap().await?;
                    // Connect to bootstrap nodes.
                    node.bootstrap().await?;
                    tracing::debug!(
                        "Bootstrapped IPFS node with default addresses: {:?}",
                        addresses
                    );
                }
                let hermes_node: HermesIpfs = node.into();
                // Enable DHT server mode for PubSub support
                hermes_node
                    .dht_mode(hermes_ipfs::rust_ipfs::DhtMode::Server)
                    .await?;
                tracing::debug!("IPFS node set to DHT server mode");
                let h = tokio::spawn(ipfs_command_handler(hermes_node, receiver));
                let (..) = tokio::join!(h);
                Ok::<(), anyhow::Error>(())
            });
            std::process::exit(0);
        });
        Ok(Self {
            sender: Some(sender),
            apps: AppIpfsState::new(),
            _phantom_data: PhantomData,
        })
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
    fn file_add(
        &self,
        contents: IpfsFile,
    ) -> Result<hermes_ipfs::IpfsPath, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::FileAddError)?
            .blocking_send(IpfsCommand::AddFile(
                AddIpfsFile::Stream((None, contents)),
                cmd_tx,
            ))
            .map_err(|_| Errno::FileAddError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::FileAddError)?
    }

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
    pub(crate) fn file_get(
        &self,
        ipfs_path: &IpfsPath,
    ) -> Result<IpfsFile, Errno> {
        let ipfs_path = BaseIpfsPath::from_str(ipfs_path).map_err(|_| Errno::InvalidIpfsPath)?;
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::FileGetError)?
            .blocking_send(IpfsCommand::GetFile(ipfs_path.clone(), cmd_tx))
            .map_err(|_| Errno::FileGetError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::FileGetError)?
    }

    /// Pin file
    ///
    /// ## Parameters
    /// - `ipfs_path`: The IPFS path of the file
    ///
    /// ## Errors
    /// - `Errno::InvalidCid`: Invalid CID
    /// - `Errno::InvalidIpfsPath`: Invalid IPFS path
    /// - `Errno::FilePinError`: Failed to pin the file
    fn file_pin(
        &self,
        ipfs_path: &IpfsPath,
    ) -> Result<bool, Errno> {
        let ipfs_path = BaseIpfsPath::from_str(ipfs_path).map_err(|_| Errno::InvalidIpfsPath)?;
        let cid = ipfs_path.root().cid().ok_or(Errno::InvalidCid)?;
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::FilePinError)?
            .blocking_send(IpfsCommand::PinFile(*cid, cmd_tx))
            .map_err(|_| Errno::FilePinError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::FilePinError)?
    }

    /// Un-in file
    ///
    /// ## Parameters
    /// - `ipfs_path`: The IPFS path of the file
    ///
    /// ## Errors
    /// - `Errno::InvalidCid`: Invalid CID
    /// - `Errno::InvalidIpfsPath`: Invalid IPFS path
    /// - `Errno::FilePinError`: Failed to pin the file
    fn file_unpin(
        &self,
        ipfs_path: &IpfsPath,
    ) -> Result<bool, Errno> {
        let ipfs_path = BaseIpfsPath::from_str(ipfs_path).map_err(|_| Errno::InvalidIpfsPath)?;
        let cid = ipfs_path.root().cid().ok_or(Errno::InvalidCid)?;
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::FilePinError)?
            .blocking_send(IpfsCommand::UnPinFile(*cid, cmd_tx))
            .map_err(|_| Errno::FilePinError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::FilePinError)?
    }

    /// Put DHT Key-Value
    fn dht_put(
        &self,
        key: DhtKey,
        value: DhtValue,
    ) -> Result<bool, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::DhtPutError)?
            .blocking_send(IpfsCommand::PutDhtValue(key, value, cmd_tx))
            .map_err(|_| Errno::DhtPutError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::DhtPutError)?
    }

    /// Get DHT Value by Key
    fn dht_get(
        &self,
        key: DhtKey,
    ) -> Result<DhtValue, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::DhtGetError)?
            .blocking_send(IpfsCommand::GetDhtValue(key, cmd_tx))
            .map_err(|_| Errno::DhtGetError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::DhtGetError)?
    }

    /// Publish message to a `PubSub` topic
    fn pubsub_publish(
        &self,
        topic: PubsubTopic,
        message: MessageData,
    ) -> Result<(), Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::PubsubPublishError)?
            .blocking_send(IpfsCommand::Publish(topic, message, cmd_tx))
            .map_err(|_| Errno::PubsubPublishError)?;
        cmd_rx
            .blocking_recv()
            .map_err(|_| Errno::PubsubPublishError)?
    }

    /// Subscribe to a `PubSub` topic
    fn pubsub_subscribe(
        &self,
        kind: SubscriptionKind,
        topic: &PubsubTopic,
    ) -> Result<JoinHandle<()>, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::PubsubSubscribeError)?
            .blocking_send(IpfsCommand::Subscribe(topic.clone(), kind, cmd_tx))
            .map_err(|_| Errno::PubsubSubscribeError)?;
        cmd_rx
            .blocking_recv()
            .map_err(|_| Errno::PubsubSubscribeError)?
    }

    /// Evict peer
    fn peer_evict(
        &self,
        peer: &PeerId,
    ) -> Result<bool, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::PeerEvictionError)?
            .blocking_send(IpfsCommand::EvictPeer(peer.clone(), cmd_tx))
            .map_err(|_| Errno::PeerEvictionError)?;
        cmd_rx
            .blocking_recv()
            .map_err(|_| Errno::PeerEvictionError)?
    }
}

/// IPFS app state
struct AppIpfsState {
    /// List of pinned files per app.
    pinned_files: DashMap<ApplicationName, HashSet<Cid>>,
    /// List of DHT values per app.
    dht_keys: DashMap<ApplicationName, HashSet<DhtKey>>,
    /// List of subscriptions per app.
    topic_subscriptions: DashMap<PubsubTopic, HashSet<ApplicationName>>,
    /// List of subscriptions per app (Doc Sync).
    doc_sync_topic_subscriptions: DashMap<PubsubTopic, HashSet<ApplicationName>>,
    /// Collection of stream join handles per topic subscription.
    subscriptions_streams: DashMap<PubsubTopic, JoinHandle<()>>,
    /// Collection of stream join handles per topic subscription (Doc Sync).
    doc_sync_subscriptions_streams: DashMap<PubsubTopic, JoinHandle<()>>,
    /// List of evicted peers per app.
    evicted_peers: DashMap<ApplicationName, HashSet<PeerId>>,
}

impl AppIpfsState {
    /// Create new `AppIpfsState`
    fn new() -> Self {
        Self {
            pinned_files: DashMap::default(),
            dht_keys: DashMap::default(),
            topic_subscriptions: DashMap::default(),
            doc_sync_topic_subscriptions: DashMap::default(),
            subscriptions_streams: DashMap::default(),
            doc_sync_subscriptions_streams: DashMap::default(),
            evicted_peers: DashMap::default(),
        }
    }

    /// Keep track of `ipfs_path` of file pinned by an app.
    fn pinned_file(
        &self,
        app_name: ApplicationName,
        ipfs_path: &IpfsPath,
    ) -> Result<(), Errno> {
        let ipfs_path: BaseIpfsPath = ipfs_path.parse().map_err(|_| Errno::InvalidIpfsPath)?;
        let cid = ipfs_path.root().cid().ok_or(Errno::InvalidCid)?;
        self.pinned_files
            .entry(app_name)
            .or_default()
            .value_mut()
            .insert(*cid);
        Ok(())
    }

    /// Un-pin a file with `ipfs_path` pinned by an app.
    fn unpinned_file(
        &self,
        app_name: &ApplicationName,
        ipfs_path: &IpfsPath,
    ) -> Result<(), Errno> {
        let ipfs_path: BaseIpfsPath = ipfs_path.parse().map_err(|_| Errno::InvalidIpfsPath)?;
        let cid = ipfs_path.root().cid().ok_or(Errno::InvalidCid)?;
        self.pinned_files
            .entry(app_name.clone())
            .or_default()
            .value_mut()
            .remove(cid);
        self.pinned_files.remove_if(app_name, |_, v| v.is_empty());
        Ok(())
    }

    /// List of pinned files by an app.
    pub(crate) fn list_pinned_files(
        &self,
        app_name: &ApplicationName,
    ) -> Vec<String> {
        self.pinned_files.get(app_name).map_or(vec![], |cids| {
            cids.iter().map(std::string::ToString::to_string).collect()
        })
    }

    /// Keep track of `dht_key` of DHT value added by an app.
    fn added_dht_key(
        &self,
        app_name: ApplicationName,
        dht_key: DhtKey,
    ) {
        self.dht_keys
            .entry(app_name)
            .or_default()
            .value_mut()
            .insert(dht_key);
    }

    /// Keep track of `topic` subscription added by an app.
    fn added_app_topic_subscription(
        &self,
        kind: SubscriptionKind,
        app_name: ApplicationName,

        topic: PubsubTopic,
    ) {
        let collection = match kind {
            SubscriptionKind::Default => &self.topic_subscriptions,
            SubscriptionKind::DocSync => &self.doc_sync_topic_subscriptions,
        };
        collection
            .entry(topic)
            .or_default()
            .value_mut()
            .insert(app_name);
    }

    /// Keep track of `topic` stream handle.
    fn added_topic_stream(
        &self,
        kind: SubscriptionKind,
        topic: PubsubTopic,

        handle: JoinHandle<()>,
    ) {
        let collection = match kind {
            SubscriptionKind::Default => &self.subscriptions_streams,
            SubscriptionKind::DocSync => &self.doc_sync_subscriptions_streams,
        };
        collection.entry(topic).insert(handle);
    }

    /// Check if a topic subscription already exists.
    fn topic_subscriptions_contains(
        &self,
        kind: SubscriptionKind,
        topic: &PubsubTopic,
    ) -> bool {
        let collection = match kind {
            SubscriptionKind::Default => &self.topic_subscriptions,
            SubscriptionKind::DocSync => &self.doc_sync_topic_subscriptions,
        };
        collection.contains_key(topic)
    }

    /// Returns a list of apps subscribed to a topic.
    fn subscribed_apps(
        &self,
        kind: SubscriptionKind,
        topic: &PubsubTopic,
    ) -> Vec<ApplicationName> {
        let collection = match kind {
            SubscriptionKind::Default => &self.topic_subscriptions,
            SubscriptionKind::DocSync => &self.doc_sync_topic_subscriptions,
        };
        collection
            .get(topic)
            .map_or(vec![], |apps| apps.value().iter().cloned().collect())
    }

    /// Add `peer_id` of evicted peer by an app.
    fn evicted_peer(
        &self,
        app_name: ApplicationName,
        peer_id: PeerId,
    ) {
        self.evicted_peers
            .entry(app_name)
            .or_default()
            .value_mut()
            .insert(peer_id);
    }
}

/// Checks for `DhtKey`, and `DhtValue` validity.
fn is_valid_dht_content(
    _key: &DhtKey,
    value: &DhtValue,
) -> bool {
    // TODO(anyone): https://github.com/input-output-hk/hermes/issues/288
    !value.is_empty()
}

/// Checks for `PubsubTopic`, and `MessageData` validity.
fn is_valid_pubsub_content(
    _topic: &PubsubTopic,
    message: &MessageData,
) -> bool {
    // TODO(anyone): https://github.com/input-output-hk/hermes/issues/288
    !message.is_empty()
}
