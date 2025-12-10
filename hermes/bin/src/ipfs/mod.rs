//! Hermes IPFS service.
mod api;
mod task;

use std::{
    collections::HashSet, convert::Infallible, marker::PhantomData, path::Path, str::FromStr,
};

pub(crate) use api::{
    hermes_ipfs_add_file, hermes_ipfs_content_validate, hermes_ipfs_dht_get_providers,
    hermes_ipfs_dht_provide, hermes_ipfs_evict_peer, hermes_ipfs_get_dht_value,
    hermes_ipfs_get_file, hermes_ipfs_get_peer_identity, hermes_ipfs_pin_file, hermes_ipfs_publish,
    hermes_ipfs_put_dht_value, hermes_ipfs_subscribe, hermes_ipfs_unpin_file,
};
use dashmap::DashMap;
use hermes_ipfs::{
    AddIpfsFile, Cid, HermesIpfs, HermesIpfsBuilder, IpfsPath as BaseIpfsPath,
    rust_ipfs::{Keypair, dummy},
};
use once_cell::sync::OnceCell;
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

/// Load an existing keypair from a file or generate a new one.
///
/// ## Parameters
///
/// * `keypair_path` - Path where the keypair should be stored
///
/// ## Returns
///
/// The loaded or newly generated keypair.
///
/// ## Errors
///
/// Returns errors if file I/O fails.
fn load_or_generate_keypair(keypair_path: &Path) -> anyhow::Result<Keypair> {
    if keypair_path.exists() {
        tracing::info!("Loading existing IPFS keypair from: {}", keypair_path.display());
        let bytes = std::fs::read(keypair_path)?;
        let keypair = Keypair::from_protobuf_encoding(&bytes)
            .map_err(|e| anyhow::anyhow!("Failed to decode keypair: {}", e))?;

        // Log the peer ID for debugging
        let peer_id = keypair.public().to_peer_id();
        tracing::info!("Loaded keypair with Peer ID: {}", peer_id);

        Ok(keypair)
    } else {
        tracing::info!("Generating new IPFS keypair at: {}", keypair_path.display());
        let keypair = Keypair::generate_ed25519();

        // Log the peer ID for debugging
        let peer_id = keypair.public().to_peer_id();
        tracing::info!("Generated new keypair with Peer ID: {}", peer_id);

        // Save the keypair for future use
        let bytes = keypair.to_protobuf_encoding()
            .map_err(|e| anyhow::anyhow!("Failed to encode keypair: {}", e))?;
        std::fs::write(keypair_path, bytes)?;
        tracing::info!("Saved keypair to: {}", keypair_path.display());

        Ok(keypair)
    }
}

/// Retry bootstrap connections in the background.
///
/// Periodically attempts to reconnect to failed peers until all are connected or max retries reached.
async fn retry_bootstrap_connections(
    node: hermes_ipfs::Ipfs,
    mut failed_peers: Vec<(String, hermes_ipfs::Multiaddr)>,
) {
    const RETRY_INTERVAL_SECS: u64 = 10;
    const MAX_RETRIES: u32 = 10;

    for attempt in 1..=MAX_RETRIES {
        if failed_peers.is_empty() {
            break;
        }

        tokio::time::sleep(std::time::Duration::from_secs(RETRY_INTERVAL_SECS)).await;
        tracing::debug!("Bootstrap retry {}/{}: attempting {} peer(s)", attempt, MAX_RETRIES, failed_peers.len());

        let mut still_failed = Vec::new();
        for (addr, multiaddr) in failed_peers {
            match node.connect(multiaddr.clone()).await {
                Ok(_) => {
                    tracing::info!("✓ Bootstrap retry succeeded: {}", addr);
                },
                Err(_) => {
                    still_failed.push((addr, multiaddr));
                },
            }
        }
        failed_peers = still_failed;
    }

    if failed_peers.is_empty() {
        tracing::info!("✓ All bootstrap peers connected");
    } else {
        tracing::warn!("⚠ {} bootstrap peer(s) still unreachable after {} retries", failed_peers.len(), MAX_RETRIES);
    }
}

/// Bootstrap `HERMES_IPFS` node.
///
/// ## Parameters
///
/// * `base_dir` - Base directory for IPFS data storage
/// * `default_bootstrap` - Whether to use default public IPFS bootstrap nodes
/// * `custom_peers` - Optional list of custom bootstrap peer multiaddrs
///
/// ## Errors
///
/// Returns errors if IPFS node fails to start.
pub fn bootstrap(
    base_dir: &Path,
    default_bootstrap: bool,
    custom_peers: Option<Vec<String>>,
) -> anyhow::Result<()> {
    let ipfs_data_path = base_dir.join("ipfs");
    if !ipfs_data_path.exists() {
        tracing::info!("creating IPFS repo directory: {}", ipfs_data_path.display());
        std::fs::create_dir_all(&ipfs_data_path)?;
    }

    // Load or generate persistent keypair
    let keypair_path = ipfs_data_path.join("keypair");
    let keypair = load_or_generate_keypair(&keypair_path)?;

    let ipfs_node = HermesIpfsNode::init(
        HermesIpfsBuilder::with_keypair(keypair)
            .map_err(|e| anyhow::anyhow!("Failed to create IPFS builder with keypair: {}", e))?
            .enable_tcp()
            .enable_quic()
            .enable_dns()
            .with_default()
            .set_disk_storage(ipfs_data_path.clone()),
        default_bootstrap,
        custom_peers,
    )?;
    HERMES_IPFS
        .set(ipfs_node)
        .map_err(|_| anyhow::anyhow!("failed to start IPFS node"))?;

    // Auto-subscribe to documents.new for P2P mesh formation
    // This ensures all nodes are subscribed on startup, allowing the Gossipsub
    // mesh to form immediately for the documents.new topic (requires mesh_n=6 peers)
    let app_name = ApplicationName::new("athena");
    let topic = PubsubTopic::from("documents.new".to_string());
    hermes_ipfs_subscribe(&app_name, topic)?;
    tracing::info!("Auto-subscribed to documents.new topic for P2P mesh formation");

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
        custom_peers: Option<Vec<String>>,
    ) -> anyhow::Result<Self> {
        let runtime = Builder::new_current_thread().enable_all().build()?;
        let (sender, receiver) = mpsc::channel(1);

        // Build and start IPFS node, before moving into the thread
        let node = runtime.block_on(async move { builder.start().await })?;

        let _handle = std::thread::spawn(move || {
            let result = runtime.block_on(async move {
                // Configure listening address for P2P connections
                match "/ip4/0.0.0.0/tcp/4001".parse() {
                    Ok(multiaddr) => {
                        match node.add_listening_address(multiaddr).await {
                            Ok(addr) => tracing::info!("IPFS listening on: {}", addr),
                            Err(e) => tracing::error!("Failed to listen on port 4001: {}", e),
                        }
                    },
                    Err(e) => tracing::error!("Invalid multiaddr format: {}", e),
                }

                // Connect to custom bootstrap peers if provided
                if let Some(peers) = custom_peers {
                    tracing::info!("Connecting to {} custom bootstrap peer(s)", peers.len());

                    let mut connected = 0;
                    let mut failed = Vec::new();

                    for peer_addr in &peers {
                        match peer_addr.parse::<hermes_ipfs::Multiaddr>() {
                            Ok(multiaddr) => {
                                match node.connect(multiaddr.clone()).await {
                                    Ok(_) => {
                                        tracing::info!("✓ Connected to bootstrap peer: {}", peer_addr);
                                        connected += 1;
                                    },
                                    Err(e) => {
                                        tracing::warn!("⚠ Initial connection failed for {}: {}", peer_addr, e);
                                        failed.push((peer_addr.clone(), multiaddr));
                                    },
                                }
                            },
                            Err(e) => {
                                tracing::warn!("⚠ Invalid multiaddr {}: {}", peer_addr, e);
                            },
                        }
                    }

                    tracing::info!("Custom bootstrap: connected to {}/{} peers", connected, peers.len());

                    // Spawn retry task for failed peers
                    if !failed.is_empty() {
                        tokio::spawn(retry_bootstrap_connections(node.clone(), failed));
                    }
                }
                // Only use public bootstrap if no custom peers provided
                else if default_bootstrap {
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
            if let Err(e) = result {
                tracing::error!("IPFS thread error: {}", e);
            }
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

    /// Provide a DHT value
    fn dht_provide(
        &self,
        key: DhtKey,
    ) -> Result<(), Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::DhtProvideError)?
            .blocking_send(IpfsCommand::DhtProvide(key, cmd_tx))
            .map_err(|_| Errno::DhtProvideError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::DhtProvideError)?
    }

    /// Get providers of a DHT value
    fn dht_get_providers(
        &self,
        key: DhtKey,
    ) -> Result<HashSet<hermes_ipfs::PeerId>, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::DhtGetProvidersError)?
            .blocking_send(IpfsCommand::DhtGetProviders(key, cmd_tx))
            .map_err(|_| Errno::DhtGetProvidersError)?;
        cmd_rx
            .blocking_recv()
            .map_err(|_| Errno::DhtGetProvidersError)?
    }

    /// Get the peer identity
    // TODO[rafal-ch]: We should not be using API errors here.
    fn get_peer_identity(&self) -> Result<hermes_ipfs::PeerInfo, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::GetPeerIdError)?
            .blocking_send(IpfsCommand::Identity(None, cmd_tx))
            .map_err(|_| Errno::GetPeerIdError)?;
        cmd_rx.blocking_recv().map_err(|_| Errno::GetPeerIdError)?
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
        topic: &PubsubTopic,
    ) -> Result<JoinHandle<()>, Errno> {
        let (cmd_tx, cmd_rx) = oneshot::channel();
        self.sender
            .as_ref()
            .ok_or(Errno::PubsubSubscribeError)?
            .blocking_send(IpfsCommand::Subscribe(topic.clone(), cmd_tx))
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
    /// Collection of stream join handles per topic subscription.
    subscriptions_streams: DashMap<PubsubTopic, JoinHandle<()>>,
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
            subscriptions_streams: DashMap::default(),
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
        app_name: ApplicationName,
        topic: PubsubTopic,
    ) {
        self.topic_subscriptions
            .entry(topic)
            .or_default()
            .value_mut()
            .insert(app_name);
    }

    /// Keep track of `topic` stream handle.
    fn added_topic_stream(
        &self,
        topic: PubsubTopic,
        handle: JoinHandle<()>,
    ) {
        self.subscriptions_streams.entry(topic).insert(handle);
    }

    /// Check if a topic subscription already exists.
    fn topic_subscriptions_contains(
        &self,
        topic: &PubsubTopic,
    ) -> bool {
        self.topic_subscriptions.contains_key(topic)
    }

    /// Returns a list of apps subscribed to a topic.
    fn subscribed_apps(
        &self,
        topic: &PubsubTopic,
    ) -> Vec<ApplicationName> {
        self.topic_subscriptions
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
