//! Hermes IPFS service.
mod api;
mod task;

use std::{
    collections::HashSet, convert::Infallible, marker::PhantomData, path::Path, str::FromStr,
};

/// Default IPFS listening port (configurable via `IPFS_LISTEN_PORT` env var)
const DEFAULT_IPFS_LISTEN_PORT: u16 = 4001;

/// Default retry interval in seconds for bootstrap connections (configurable via
/// `IPFS_RETRY_INTERVAL_SECS`)
const DEFAULT_RETRY_INTERVAL_SECS: u64 = 10;

/// Default maximum retry attempts for bootstrap connections (configurable via
/// `IPFS_MAX_RETRIES`)
const DEFAULT_MAX_RETRIES: u32 = 10;

/// IPFS data subdirectory name within the base directory
const IPFS_DATA_DIR: &str = "ipfs";

/// Keypair filename for persistent IPFS identity
const KEYPAIR_FILENAME: &str = "keypair";

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
#[derive(Clone, Debug)]
pub struct Config<'a> {
    /// Base directory for IPFS data storage
    pub base_dir: &'a Path,
    /// Whether to use default public IPFS bootstrap nodes
    pub default_bootstrap: bool,
    /// Optional list of custom bootstrap peer multiaddrs
    pub custom_peers: Option<Vec<String>>,
}

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
        tracing::info!(
            "Loading existing IPFS keypair from: {}",
            keypair_path.display()
        );
        let bytes = std::fs::read(keypair_path)?;
        let keypair = Keypair::from_protobuf_encoding(&bytes)
            .map_err(|e| anyhow::anyhow!("Failed to decode keypair: {e}"))?;

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
        let bytes = keypair
            .to_protobuf_encoding()
            .map_err(|e| anyhow::anyhow!("Failed to encode keypair: {e}"))?;
        std::fs::write(keypair_path, bytes)?;
        tracing::info!("Saved keypair to: {}", keypair_path.display());

        Ok(keypair)
    }
}

/// Parse environment variable with fallback to default value.
fn env_var_or<T: std::str::FromStr>(
    key: &str,
    default: T,
) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Configure IPFS node to listen on the specified port.
async fn configure_listening_address(node: &hermes_ipfs::Ipfs) {
    let listen_port = env_var_or("IPFS_LISTEN_PORT", DEFAULT_IPFS_LISTEN_PORT);
    let listen_addr = format!("/ip4/0.0.0.0/tcp/{listen_port}");

    match listen_addr.parse() {
        Ok(multiaddr) => {
            match node.add_listening_address(multiaddr).await {
                Ok(addr) => tracing::info!("IPFS listening on: {}", addr),
                Err(e) => tracing::error!("Failed to listen on port {}: {}", listen_port, e),
            }
        },
        Err(e) => tracing::error!("Invalid multiaddr format: {}", e),
    }
}

/// Connect to custom bootstrap peers and retry failed connections in the background.
///
/// ## What are bootstrap peers?
///
/// Bootstrap peers are "address book entries" - initial contact points for joining the
/// P2P network. Think of them like DNS servers for the internet: without a starting
/// point, your node can't find anything.
///
/// They provide:
/// - DHT entry points: Access to the distributed routing table for finding content/peers
/// - Peer discovery: Learn about other nodes in the network through gossip protocols
/// - Gossipsub mesh: Enable `PubSub` message propagation by connecting to topic
///   subscribers
///
/// Without bootstrap peers, a node is isolated - it won't discover peers, can't query the
/// DHT, and can't participate in `PubSub` topics.
///
/// ## Returns
///
/// The number of successfully connected peers.
async fn connect_to_bootstrap_peers(
    node: &hermes_ipfs::Ipfs,
    peers: Vec<String>,
) -> usize {
    tracing::info!("Connecting to {} custom bootstrap peer(s)", peers.len());

    let mut connected: usize = 0;
    let mut failed = Vec::new();

    for peer_addr in &peers {
        match peer_addr.parse::<hermes_ipfs::Multiaddr>() {
            Ok(multiaddr) => {
                match node.connect(multiaddr.clone()).await {
                    Ok(_) => {
                        tracing::info!("✓ Connected to bootstrap peer: {}", peer_addr);
                        connected = connected.saturating_add(1);
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

    tracing::info!(
        "Custom bootstrap: connected to {}/{} peers",
        connected,
        peers.len()
    );

    // Spawn retry task for failed peers
    if !failed.is_empty() {
        tokio::spawn(retry_bootstrap_connections(node.clone(), failed));
    }

    connected
}

/// Retry bootstrap connections in the background.
///
/// ## Retry Strategy
///
/// Uses a simple fixed-interval retry approach:
/// - Waits `RETRY_INTERVAL_SECS` between attempts (configurable, default: 10s)
/// - Retries up to `MAX_RETRIES` times (configurable, default: 10 attempts)
/// - Stops early if all peers connect successfully
///
/// This ensures eventual connectivity even when peers are temporarily unreachable
/// (e.g., network delays, node startup timing issues in test environments).
///
/// Periodically attempts to reconnect to failed peers until all are connected or max
/// retries reached.
async fn retry_bootstrap_connections(
    node: hermes_ipfs::Ipfs,
    mut failed_peers: Vec<(String, hermes_ipfs::Multiaddr)>,
) {
    if failed_peers.is_empty() {
        return;
    }

    let retry_interval = std::time::Duration::from_secs(env_var_or(
        "IPFS_RETRY_INTERVAL_SECS",
        DEFAULT_RETRY_INTERVAL_SECS,
    ));
    let max_retries = env_var_or("IPFS_MAX_RETRIES", DEFAULT_MAX_RETRIES);

    for attempt in 1..=max_retries {
        tokio::time::sleep(retry_interval).await;
        tracing::debug!(
            "Bootstrap retry {}/{}: attempting {} peer(s)",
            attempt,
            max_retries,
            failed_peers.len()
        );

        let mut still_failed = Vec::new();
        for (addr, multiaddr) in failed_peers {
            if node.connect(multiaddr.clone()).await.is_err() {
                still_failed.push((addr, multiaddr));
            } else {
                tracing::info!("✓ Bootstrap retry succeeded: {}", addr);
            }
        }
        failed_peers = still_failed;

        if failed_peers.is_empty() {
            tracing::info!("✓ All bootstrap peers connected");
            return;
        }
    }

    tracing::warn!(
        "⚠ {} bootstrap peer(s) still unreachable after {} retries",
        failed_peers.len(),
        max_retries
    );
}

/// Bootstrap `HERMES_IPFS` node.
///
/// ## What is bootstrapping?
///
/// Bootstrapping connects an IPFS node to the network by:
/// 1. Loading/generating a persistent keypair (stable peer identity)
/// 2. Configuring network transports (TCP, QUIC, DNS)
/// 3. Connecting to initial bootstrap peers (entry points to the network)
/// 4. Auto-subscribing to a default topic for immediate mesh participation
///
/// ## IMPORTANT: `PubSub` requires custom bootstrap peers
///
/// **TL;DR: Public IPFS nodes don't work for Hermes `PubSub`. Use custom Hermes bootstrap
/// peers.**
///
/// **Why public IPFS bootstrap nodes CANNOT be used for Hermes `PubSub`:**
///
/// Gossipsub (the `PubSub` protocol) requires ALL peers in the mesh to:
/// 1. Subscribe to the **same topic** (e.g., "documents.new")
/// 2. Be connected to each other in mesh topology
///
/// Public IPFS nodes:
/// - Don't subscribe to Hermes-specific topics → can't propagate your messages
/// - Only provide DHT routing and general peer discovery
/// - Are useless for `PubSub` message exchange
///
/// **For `PubSub` to work, you MUST:**
/// - Use `custom_peers` pointing to other Hermes nodes that subscribe to your topics
/// - OR deploy dedicated Hermes bootstrap nodes configured to auto-subscribe
///
/// The `default_bootstrap` option is ONLY useful for:
/// - File storage (IPFS add/get operations)
/// - DHT queries (finding content providers)
/// - General peer discovery
///
/// It will NOT enable `PubSub` message propagation.
///
/// ## Parameters
///
/// * `base_dir` - Base directory for IPFS data storage
/// * `default_bootstrap` - Whether to use default public IPFS bootstrap nodes
/// * `custom_peers` - Optional list of custom bootstrap peer multiaddrs
///
/// ## What is bootstrapping?
///
/// Bootstrapping connects an IPFS node to the network by:
/// 1. Loading/generating a persistent keypair (stable peer identity)
/// 2. Configuring network transports (TCP, QUIC, DNS)
/// 3. Connecting to initial bootstrap peers (entry points to the network)
/// 4. Auto-subscribing to a default topic for immediate mesh participation
///
/// ## IMPORTANT: `PubSub` requires custom bootstrap peers
///
/// **TL;DR: Public IPFS nodes don't work for Hermes `PubSub`. Use custom Hermes bootstrap
/// peers.**
///
/// **Why public IPFS bootstrap nodes CANNOT be used for Hermes `PubSub`:**
///
/// Gossipsub (the `PubSub` protocol) requires ALL peers in the mesh to:
/// 1. Subscribe to the **same topic** (e.g., "documents.new")
/// 2. Be connected to each other in mesh topology
///
/// Public IPFS nodes:
/// - Don't subscribe to Hermes-specific topics → can't propagate your messages
/// - Only provide DHT routing and general peer discovery
/// - Are useless for `PubSub` message exchange
///
/// **For `PubSub` to work, you MUST:**
/// - Use `custom_peers` pointing to other Hermes nodes that subscribe to your topics
/// - OR deploy dedicated Hermes bootstrap nodes configured to auto-subscribe
///
/// The `default_bootstrap` option is ONLY useful for:
/// - File storage (IPFS add/get operations)
/// - DHT queries (finding content providers)
/// - General peer discovery
///
/// It will NOT enable `PubSub` message propagation.
///
/// ## Parameters
///
/// * `config` - IPFS bootstrap config
///
/// ## Errors
///
/// Returns errors if IPFS node fails to start.
pub fn bootstrap(config: Config) -> anyhow::Result<()> {
    let ipfs_data_path = config.base_dir.join(IPFS_DATA_DIR);
    if !ipfs_data_path.exists() {
        tracing::info!("creating IPFS repo directory: {}", ipfs_data_path.display());
        std::fs::create_dir_all(&ipfs_data_path)?;
    }

    // Load or generate persistent keypair
    let keypair_path = ipfs_data_path.join(KEYPAIR_FILENAME);
    let keypair = load_or_generate_keypair(&keypair_path)?;

    let ipfs_node = HermesIpfsNode::init(
        HermesIpfsBuilder::with_keypair(keypair)
            .map_err(|e| anyhow::anyhow!("Failed to create IPFS builder with keypair: {e}"))?
            .enable_tcp()
            .enable_quic()
            .enable_dns()
            .with_default()
            .set_disk_storage(ipfs_data_path.clone()),
        config.default_bootstrap,
        config.custom_peers,
    )?;
    HERMES_IPFS
        .set(ipfs_node)
        .map_err(|_| anyhow::anyhow!("failed to start IPFS node"))?;

    // =========================================================================
    // CHANGE: Removed auto-subscription logic (2025-12-21)
    // =========================================================================
    //
    // WHAT WAS REMOVED:
    // - Auto-subscription to "documents.new" topic during IPFS bootstrap
    // - Constants: DEFAULT_APP_NAME and DEFAULT_MESH_TOPIC
    //
    // WHY IT WAS REMOVED:
    // - The auto-subscription was causing subscription conflicts
    // - When the doc-sync module tried to subscribe with SubscriptionKind::DocSync,
    //   the topic was already subscribed with SubscriptionKind::Default
    // - IPFS PubSub only allows one subscription per topic per node
    // - This prevented the doc-sync handler from receiving messages
    //
    // NEW BEHAVIOR:
    // - P2P mesh formation now happens when modules subscribe to their topics
    // - The doc-sync module subscribes to "documents.new" during initialization
    // - This uses SubscriptionKind::DocSync which routes to doc_sync_topic_message_handler
    // - Each module manages its own subscriptions without conflicts
    //
    // IMPORTANT: If you add this auto-subscription back, ensure it uses the
    // correct SubscriptionKind for the handler you want to use, or move to a
    // different topic that doesn't conflict with module-specific subscriptions.
    // =========================================================================

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

        // Create a oneshot channel to signal when the command handler is ready
        let (ready_tx, ready_rx) = oneshot::channel();

        let _handle = std::thread::spawn(move || {
            let result = runtime.block_on(async move {
                // Configure listening address for P2P connections
                configure_listening_address(&node).await;

                // Connect to bootstrap peers
                if let Some(peers) = custom_peers {
                    connect_to_bootstrap_peers(&node, peers).await;
                } else if default_bootstrap {
                    // Use public IPFS bootstrap nodes
                    let addresses = node.default_bootstrap().await?;
                    node.bootstrap().await?;
                    tracing::debug!(
                        "Bootstrapped IPFS node with default addresses: {:?}",
                        addresses
                    );
                }

                // Why DHT Server Mode is Required:
                // - DHT (Distributed Hash Table) server mode makes this node actively participate
                //   in the DHT by storing and serving routing information
                // - This is REQUIRED for Gossipsub PubSub to work properly because:
                //   1. PubSub uses the DHT to discover which peers are subscribed to topics
                //   2. Gossipsub builds mesh connections based on DHT peer discovery
                //   3. Without server mode, the node would be a "leech" that can't help other peers
                //      discover the network, weakening the mesh
                // - All Hermes nodes should be DHT servers to form a robust P2P network
                let hermes_node: HermesIpfs = node.into();
                hermes_node
                    .dht_mode(hermes_ipfs::rust_ipfs::DhtMode::Server)
                    .await?;
                tracing::debug!("IPFS node set to DHT server mode");

                // Start command handler

                // Signal that the command handler is about to start
                // Ignore the error if the receiver was dropped
                let _ = ready_tx.send(());

                // Start command handler
                let h = tokio::spawn(ipfs_command_handler(hermes_node, receiver));
                let (..) = tokio::join!(h);
                Ok::<(), anyhow::Error>(())
            });

            if let Err(e) = result {
                tracing::error!("IPFS thread error: {}", e);
            }
        });

        // Wait for the command handler to be ready before returning
        // This prevents the race condition where auto-subscribe happens before
        // the command handler is ready to process commands
        ready_rx.blocking_recv().map_err(|_| {
            anyhow::anyhow!("IPFS initialization failed: command handler thread died")
        })?;

        tracing::debug!("IPFS command handler is ready");

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
