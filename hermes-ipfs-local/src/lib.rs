//! Hermes IPFS
//!
//! Provides support for storage, and `PubSub` functionality.

pub(crate) mod constant;

#[cfg(feature = "doc-sync")]
pub mod doc_sync;

use std::{collections::HashSet, convert::Infallible, str::FromStr};

use derive_more::{Display, From, Into};
use futures::{StreamExt, TryStreamExt, pin_mut, stream::BoxStream};
/// IPFS Content Identifier.
pub use ipld_core::cid::Cid;
/// IPLD
pub use ipld_core::ipld::Ipld;
use libp2p::gossipsub::MessageId as PubsubMessageId;
use multihash_codetable::{Code, MultihashDigest};
/// `rust_ipfs` re-export.
pub use rust_ipfs;
/// Server, Client, or Auto mode
pub use rust_ipfs::DhtMode;
/// Server, Client, or Auto mode
pub use rust_ipfs::Ipfs;
/// Multiaddr type.
pub use rust_ipfs::Multiaddr;
/// Peer ID type.
pub use rust_ipfs::PeerId;
/// Peer Info type.
pub use rust_ipfs::p2p::PeerInfo;
/// Enum for specifying paths in IPFS.
pub use rust_ipfs::path::IpfsPath;
/// Storage type for IPFS node.
pub use rust_ipfs::repo::StorageTypes;
use rust_ipfs::{
    Block, GossipsubMessage, NetworkBehaviour, Quorum, ToRecordKey, builder::IpfsBuilder,
    dag::ResolveError, dummy, gossipsub::IntoGossipsubTopic,
};

use crate::constant::CODEC_CBOR;

#[derive(Debug, Display, From, Into)]
/// `PubSub` Message ID.
pub struct MessageId(pub PubsubMessageId);

/// Builder type for IPFS Node configuration.
pub struct HermesIpfsBuilder<N>(IpfsBuilder<N>)
where N: NetworkBehaviour<ToSwarm = Infallible> + Send + Sync;

impl Default for HermesIpfsBuilder<dummy::Behaviour> {
    fn default() -> Self {
        Self(IpfsBuilder::new())
    }
}

impl<N> HermesIpfsBuilder<N>
where N: NetworkBehaviour<ToSwarm = Infallible> + Send + Sync
{
    #[must_use]
    /// Create a new` IpfsBuilder`.
    pub fn new() -> Self {
        Self(IpfsBuilder::new())
    }

    /// Create a new `IpfsBuilder` with an existing keypair.
    ///
    /// ## Parameters
    /// - `keypair`: An existing keypair (can be `libp2p::identity::Keypair` or compatible
    ///   type)
    ///
    /// ## Errors
    /// Returns an error if the keypair is invalid.
    pub fn with_keypair(keypair: impl connexa::builder::IntoKeypair) -> std::io::Result<Self> {
        Ok(Self(IpfsBuilder::with_keypair(keypair)?))
    }

    #[must_use]
    /// Set the default configuration for the IPFS node.
    pub fn with_default(self) -> Self {
        Self(self.0.with_default())
    }

    #[must_use]
    /// Set the default listener for the IPFS node.
    pub fn set_default_listener(self) -> Self {
        Self(self.0.set_default_listener())
    }

    #[must_use]
    /// Enable TCP transport.
    pub fn enable_tcp(self) -> Self {
        Self(self.0.enable_tcp())
    }

    #[must_use]
    /// Enable QUIC transport.
    pub fn enable_quic(self) -> Self {
        Self(self.0.enable_quic())
    }

    #[must_use]
    /// Enable DNS resolution.
    pub fn enable_dns(self) -> Self {
        Self(self.0.enable_dns())
    }

    #[must_use]
    /// Set the storage type for the IPFS node to local disk.
    ///
    /// ## Parameters
    pub fn set_disk_storage<T: Into<std::path::PathBuf>>(
        self,
        storage_path: T,
    ) -> Self {
        Self(
            self.0
                .set_repo(&rust_ipfs::repo::Repo::new_fs(storage_path.into())),
        )
    }

    /// Start the IPFS node.
    ///
    /// ## Errors
    /// Returns an error if the IPFS daemon fails to start.
    pub async fn start(self) -> anyhow::Result<Ipfs> {
        self.0.start().await
    }
}

/// Hermes IPFS Node.
pub struct HermesIpfs {
    /// IPFS node
    node: Ipfs,
}

impl HermesIpfs {
    /// Start a new node.
    ///
    /// ## Returns
    ///
    /// * `HermesIpfs`
    ///
    /// ## Errors
    ///
    /// Returns an error if the IPFS daemon fails to start.
    pub async fn start() -> anyhow::Result<Self> {
        let node = HermesIpfsBuilder::<dummy::Behaviour>::new()
            .enable_tcp()
            .enable_quic()
            .enable_dns()
            .with_default()
            .set_default_listener()
            // TODO(saibatizoku): Re-Enable default transport config when libp2p Cert bug is fixed
            // TODO(rafal-ch): TLS is disabled by default, we can enable it by calling
            // on of the `IpfsBuilder::enable_secure...()` flavors.
            //.enable_secure_websocket()
            .start()
            .await?;
        Ok(HermesIpfs { node })
    }

    /// Add a file to IPFS by creating a block
    /// The CID is generated using
    /// - Codec: CBOR 0x51
    /// - CBOR encoded data
    /// - Hash function: SHA2-256
    ///
    /// ## Parameters
    ///
    /// * `data` - `Vec<u8>` Data to be uploaded.
    ///
    /// ## Returns
    ///
    /// * A result with `IpfsPath`
    ///
    /// ## Errors
    ///
    /// Returns an error if the block fails to upload.
    pub async fn add_ipfs_file(
        &self,
        data: Vec<u8>,
    ) -> anyhow::Result<IpfsPath> {
        let cbor_data = minicbor::to_vec(data)
            .map_err(|e| anyhow::anyhow!("Failed to encode data to CBOR: {e:?}"))?;
        let cid = Cid::new_v1(CODEC_CBOR.into(), Code::Sha2_256.digest(&cbor_data));
        let block = Block::new(cid, cbor_data)
            .map_err(|e| anyhow::anyhow!("Failed to create IPFS block: {e:?}"))?;
        let ipfs_path: IpfsPath = self.node.put_block(&block).await?.into();
        Ok(ipfs_path)
    }

    /// Get a file from IPFS as CBOR encoded data.
    ///
    /// ## Parameters
    ///
    /// * `cid` - `Cid` Content identifier to be downloaded.
    ///
    /// ## Returns
    ///
    /// * `A result with Vec<u8>`.
    ///
    /// ## Errors
    ///
    /// Returns an error if the file fails to download.
    pub async fn get_ipfs_file_cbor(
        &self,
        cid: &Cid,
    ) -> anyhow::Result<Vec<u8>> {
        let block = self.node.get_block(cid).await?;
        Ok(block.data().to_vec())
    }

    /// Get a file from IPFS as CBOR encoded data, specifying providers to fetch from.
    ///
    /// This method allows specifying known providers (peers that have the content)
    /// to avoid relying on Bitswap's NeedBlock event which requires external handling
    /// for DHT content discovery.
    ///
    /// ## Parameters
    ///
    /// * `cid` - `Cid` Content identifier to be downloaded.
    /// * `providers` - Slice of `PeerId`s known to have the content.
    ///
    /// ## Returns
    ///
    /// * `A result with Vec<u8>`.
    ///
    /// ## Errors
    ///
    /// Returns an error if the file fails to download.
    pub async fn get_ipfs_file_cbor_with_providers(
        &self,
        cid: &Cid,
        providers: &[PeerId],
    ) -> anyhow::Result<Vec<u8>> {
        use libp2p::swarm::dial_opts::DialOpts;
        use std::time::Duration;
        use tokio::time::timeout;

        // Connect to providers first before calling get_block.
        // This is a workaround for rust-ipfs Bitswap which initiates dials to providers
        // but doesn't wait for them to complete before checking connectivity.
        let mut connected_providers = Vec::new();
        for peer_id in providers {
            // Check if already connected
            if self.node.is_connected(*peer_id).await.unwrap_or(false) {
                tracing::debug!(%peer_id, "Already connected to provider");
                connected_providers.push(*peer_id);
                continue;
            }

            // Try to connect with a short timeout
            let dial_opts = DialOpts::peer_id(*peer_id).build();
            match timeout(Duration::from_secs(5), self.node.connect(dial_opts)).await {
                Ok(Ok(_)) => {
                    tracing::debug!(%peer_id, "Successfully connected to provider");
                    connected_providers.push(*peer_id);
                },
                Ok(Err(err)) => {
                    tracing::debug!(%peer_id, %err, "Failed to connect to provider");
                },
                Err(_) => {
                    tracing::debug!(%peer_id, "Timeout connecting to provider");
                },
            }
        }

        if connected_providers.is_empty() {
            anyhow::bail!("Failed to connect to any providers");
        }

        tracing::info!(
            %cid,
            total_providers = providers.len(),
            connected_providers = connected_providers.len(),
            "Fetching block with connected providers"
        );

        let block = self
            .node
            .get_block(cid)
            .providers(&connected_providers)
            .await?;
        Ok(block.data().to_vec())
    }

    /// Connect to a peer by PeerId.
    ///
    /// This initiates a dial to the peer. The peer's addresses must be known
    /// (e.g., from a previous DHT lookup or connection).
    ///
    /// ## Parameters
    ///
    /// * `peer_id` - The PeerId to connect to.
    ///
    /// ## Errors
    ///
    /// Returns an error if the connection fails.
    pub async fn connect_peer(
        &self,
        peer_id: PeerId,
    ) -> anyhow::Result<()> {
        use libp2p::swarm::dial_opts::DialOpts;
        let dial_opts = DialOpts::peer_id(peer_id).build();
        self.node.connect(dial_opts).await?;
        Ok(())
    }

    /// Check if we are connected to a peer.
    ///
    /// ## Parameters
    ///
    /// * `peer_id` - The PeerId to check.
    ///
    /// ## Returns
    ///
    /// `true` if connected, `false` otherwise.
    ///
    /// ## Errors
    ///
    /// Returns an error if the check fails.
    pub async fn is_connected(
        &self,
        peer_id: PeerId,
    ) -> anyhow::Result<bool> {
        Ok(self.node.is_connected(peer_id).await?)
    }

    /// Pin content to IPFS.
    ///
    /// ## Parameters
    ///
    /// * `cid` - `Cid` Content identifier to be pinned.
    ///
    /// ## Errors
    ///
    /// Returns an error if pinning fails.
    pub async fn insert_pin(
        &self,
        cid: &Cid,
    ) -> anyhow::Result<()> {
        self.node.insert_pin(cid).await
    }

    /// Checks whether a given block is pinned.
    ///
    /// # Crash unsafety
    ///
    /// Cannot currently detect partially written recursive pins. Those can happen if
    /// [`HermesIpfs::insert_pin`] is interrupted by a crash for example.
    ///
    /// Works correctly only under no-crash situations. Workaround for hitting a crash is
    /// to re-pin any existing recursive pins.
    ///
    /// ## Parameters
    ///
    /// * `cid` - `Cid` Content identifier to be pinned.
    ///
    /// ## Returns
    /// `true` if the block is pinned, `false` if not. See Crash unsafety notes for the
    /// false response.
    ///
    /// ## Errors
    ///
    /// Returns an error if checking pin fails.
    pub async fn is_pinned(
        &self,
        cid: &Cid,
    ) -> anyhow::Result<bool> {
        self.node.is_pinned(cid).await
    }

    /// List all pins in the IPFS node.
    ///
    /// ## Parameters
    /// * `cid` - `Option<Cid>` Optional content identifier to list pins. If `None`, lists
    ///   all pins.
    ///
    /// ## Errors
    /// Returns an error if listing pins fails.
    pub async fn list_pins(&self) -> anyhow::Result<Vec<Cid>> {
        // List all kinds of pins by setting `None` as the argument.
        let pins_stream = self.node.list_pins(None).await;
        pin_mut!(pins_stream);
        let mut pins = vec![];
        while let Some(pinned) = pins_stream.next().await {
            pins.push(pinned?.0);
        }
        Ok(pins)
    }

    /// Remove pinned content from IPFS.
    ///
    /// ## Parameters
    ///
    /// * `cid` - `Cid` Content identifier to be un-pinned.
    ///
    /// ## Errors
    ///
    /// Returns an error if removing pin fails.
    pub async fn remove_pin(
        &self,
        cid: &Cid,
    ) -> anyhow::Result<()> {
        self.node.remove_pin(cid).recursive().await
    }

    /// Stop and exit the IPFS node daemon.
    pub async fn stop(self) {
        self.node.exit_daemon().await;
    }

    /// Returns the peer identity information. If no peer id is supplied the local node
    /// identity is used.
    ///
    /// ## Parameters
    ///
    /// * `peer_id` - `Option<PeerId>`
    ///
    /// ## Errors
    ///
    /// Returns error if peer info cannot be retrieved.
    pub async fn identity(
        &self,
        peer_id: Option<PeerId>,
    ) -> anyhow::Result<PeerInfo> {
        self.node.identity(peer_id).await
    }

    /// Add peer to address book.
    ///
    /// ## Parameters
    ///
    /// * `peer_id` - `PeerId`
    /// * `addr` - `Multiaddr`
    ///
    /// ## Errors
    ///
    /// Returns error if unable to add peer.
    pub async fn add_peer(
        &self,
        peer_id: PeerId,
        addr: Multiaddr,
    ) -> anyhow::Result<()> {
        self.node.add_peer((peer_id, addr)).await
    }

    /// List of local listening addresses
    ///
    /// ## Returns
    ///
    /// * `Result<Vec<Multiaddr>>`
    ///
    /// ## Errors
    ///
    /// Returns error if listening addresses cannot be retrieved.
    pub async fn listening_addresses(&self) -> anyhow::Result<Vec<Multiaddr>> {
        self.node.listening_addresses().await
    }

    /// Sets DHT mode in the IPFS node.
    ///
    /// ## Parameters
    ///
    /// * `mode` - `DhtMode`
    ///
    /// ## Returns
    ///
    /// * `Result<()>`
    ///
    /// ## Errors
    ///
    /// Returns error if unable to set DHT mode
    pub async fn dht_mode(
        &self,
        mode: DhtMode,
    ) -> anyhow::Result<()> {
        self.node.dht_mode(mode).await
    }

    /// Add DAG data to IPFS.
    ///
    /// ## Parameters
    ///
    /// * `ipld` - `Ipld`
    ///
    /// ## Returns
    ///
    /// * `Result<Cid>`
    ///
    /// ## Errors
    ///
    /// Returns error if unable to add DAG content.
    pub async fn dag_put(
        &self,
        ipld: Ipld,
    ) -> anyhow::Result<Cid> {
        self.node.put_dag(ipld).await
    }

    /// Get DAG data from IPFS.
    ///
    /// ## Parameters
    ///
    /// * `path` - `impl Into<IpfsPath>`
    ///
    /// ## Returns
    ///
    /// * `Result<Ipld>`
    ///
    /// ## Errors
    ///
    /// Returns error if unable to get DAG content.
    pub async fn dag_get<T: Into<IpfsPath>>(
        &self,
        path: T,
    ) -> Result<Ipld, ResolveError> {
        self.node.get_dag(path).await
    }

    /// Add content to DHT.
    ///
    /// ## Parameters
    ///
    /// * `key` - `impl AsRef<[u8]>`
    /// * `value` - `impl Into<Vec<u8>>`
    ///
    /// ## Returns
    ///
    /// * `Result<()>`
    ///
    /// ## Errors
    ///
    /// Returns error if unable to add content to DHT
    pub async fn dht_put(
        &self,
        key: impl AsRef<[u8]>,
        value: impl Into<Vec<u8>>,
    ) -> anyhow::Result<()> {
        self.node.dht_put(key, value.into(), Quorum::One).await
    }

    /// Get content from DHT.
    ///
    /// ## Parameters
    ///
    /// * `key` - `impl AsRef<[u8]>`
    ///
    /// ## Returns
    ///
    /// * `Result<Vec<u8>>`
    ///
    /// ## Errors
    ///
    /// Returns error if unable to get content from DHT
    pub async fn dht_get(
        &self,
        key: impl AsRef<[u8]> + ToRecordKey,
    ) -> anyhow::Result<Vec<u8>> {
        let record_stream = self.node.dht_get(key).await?;
        pin_mut!(record_stream);
        // TODO: We only ever return a single value from the stream. We might want to improve
        // this.
        let record = record_stream
            .next()
            .await
            .ok_or(anyhow::anyhow!("No record found"))?;
        Ok(record.value)
    }

    /// Announce this node as a provider for the given DHT key.
    ///
    /// ## Parameters
    ///
    /// * `key` - Key identifying the content or resource to provide on the DHT.
    ///
    /// ## Returns
    ///
    /// * `Result<()>` — Indicates whether the provider announcement succeeded.
    ///
    /// ## Errors
    ///
    /// Returns an error if announcing provider information to the DHT fails.
    pub async fn dht_provide(
        &self,
        key: impl AsRef<[u8]> + ToRecordKey,
    ) -> anyhow::Result<()> {
        self.node.dht_provide(key).await
    }

    /// Retrieve all providers for the given DHT key.
    ///
    /// ## Parameters
    ///
    /// * `key` - Key identifying the content or resource in the DHT.
    ///
    /// ## Returns
    ///
    /// * `Result<HashSet<PeerId>>` — A set containing all `PeerId`s reported as providers
    ///   for the given key.
    ///
    /// ## Errors
    ///
    /// Returns an error if the provider stream fails or if retrieving provider
    /// information from the DHT encounters an underlying error.
    pub async fn dht_get_providers(
        &self,
        key: impl AsRef<[u8]> + ToRecordKey,
    ) -> anyhow::Result<HashSet<PeerId>> {
        Ok(self
            .node
            .dht_get_providers(key)
            .await?
            .try_fold(HashSet::new(), |mut acc, set| {
                async move {
                    acc.extend(set);
                    Ok(acc)
                }
            })
            .await?)
    }

    /// Add address to bootstrap nodes.
    ///
    /// ## Parameters
    ///
    /// * `address` - `Multiaddr`
    ///
    /// ## Returns
    ///
    /// * `Result<Multiaddr>`
    ///
    /// ## Errors
    ///
    /// Returns error if unable to add address to bootstrap nodes
    pub async fn add_bootstrap(
        &self,
        address: Multiaddr,
    ) -> anyhow::Result<Multiaddr> {
        self.node.add_bootstrap(address).await
    }

    /// Bootstrap the IPFS node.
    ///
    /// ## Returns
    ///
    /// * `Result<()>`
    ///
    /// ## Errors
    ///
    /// Returns error if unable to retrieve bootstrap the node.
    pub async fn bootstrap(&self) -> anyhow::Result<()> {
        self.node.bootstrap().await
    }

    /// Subscribes to a pubsub topic.
    ///
    /// ## Parameters
    ///
    /// * `topic` - `impl Into<String>`
    ///
    /// ## Returns
    ///
    /// * Stream of `GossipsubEvent`
    ///
    /// ## Errors
    ///
    /// Returns error if unable to subscribe to pubsub topic.
    pub async fn pubsub_subscribe(
        &self,
        topic: impl Into<String>,
    ) -> anyhow::Result<BoxStream<'static, connexa::prelude::GossipsubEvent>> {
        let topic = topic.into();
        self.node.pubsub_subscribe(&topic).await?;
        self.node.pubsub_listener(&topic).await
    }

    /// Unsubscribes from a pubsub topic.
    ///
    /// ## Parameters
    ///
    /// * `topic` - `impl Into<String>`
    ///
    /// ## Errors
    ///
    /// Returns error if unable to unsubscribe from pubsub topic.
    pub async fn pubsub_unsubscribe(
        &self,
        topic: impl Into<String> + IntoGossipsubTopic,
    ) -> anyhow::Result<()> {
        self.node.pubsub_unsubscribe(topic).await
    }

    /// Publishes a message to a pubsub topic.
    ///
    /// ## Parameters
    ///
    /// * `topic` - `impl Into<String>`
    /// * `message` - `Vec<u8>`
    ///
    /// ## Errors
    ///
    /// Returns error if unable to publish to a pubsub topic.
    pub async fn pubsub_publish(
        &self,
        topic: impl IntoGossipsubTopic,
        message: Vec<u8>,
    ) -> anyhow::Result<()> {
        self.node.pubsub_publish(topic, message).await
    }

    /// Ban peer from node.
    ///
    /// ## Parameters
    ///
    /// * `peer` - `PeerId`
    ///
    /// ## Returns
    ///
    /// * `Result<()>`
    ///
    /// ## Errors
    ///
    /// Returns error if unable to ban peer.
    pub async fn ban_peer(
        &self,
        peer: PeerId,
    ) -> anyhow::Result<()> {
        self.node.ban_peer(peer).await
    }
}

impl From<Ipfs> for HermesIpfs {
    fn from(node: Ipfs) -> Self {
        Self { node }
    }
}

/// Path to get the file from IPFS
pub struct GetIpfsFile(IpfsPath);

impl From<Cid> for GetIpfsFile {
    fn from(value: Cid) -> Self {
        GetIpfsFile(value.into())
    }
}

impl From<IpfsPath> for GetIpfsFile {
    fn from(value: IpfsPath) -> Self {
        GetIpfsFile(value)
    }
}

impl From<GetIpfsFile> for IpfsPath {
    fn from(value: GetIpfsFile) -> Self {
        value.0
    }
}

impl FromStr for GetIpfsFile {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(GetIpfsFile(s.parse()?))
    }
}

/// `GossipsubEvents` related to subscription state
#[derive(Display, Debug)]
pub enum SubscriptionStatusEvent {
    /// Peer has been subscribed
    Subscribed {
        /// Peer id
        peer_id: PeerId,
    },
    /// Peer has been unsubscribed
    Unsubscribed {
        /// Peer id
        peer_id: PeerId,
    },
}

/// Handle stream of messages from the IPFS pubsub topic
pub fn subscription_stream_task<MH, SH>(
    stream: BoxStream<'static, connexa::prelude::GossipsubEvent>,
    message_handler: MH,
    subscription_handler: SH,
) -> tokio::task::JoinHandle<()>
where
    MH: Fn(GossipsubMessage) + Send + 'static,
    SH: Fn(SubscriptionStatusEvent) + Send + 'static,
{
    tokio::spawn(async move {
        pin_mut!(stream);
        while let Some(msg) = stream.next().await {
            match msg {
                connexa::prelude::GossipsubEvent::Subscribed { peer_id } => {
                    subscription_handler(SubscriptionStatusEvent::Subscribed { peer_id });
                },
                connexa::prelude::GossipsubEvent::Unsubscribed { peer_id } => {
                    subscription_handler(SubscriptionStatusEvent::Unsubscribed { peer_id });
                },
                connexa::prelude::GossipsubEvent::Message { message } => message_handler(message),
            }
        }
    })
}
