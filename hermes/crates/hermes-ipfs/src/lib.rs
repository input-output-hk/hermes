//! Hermes IPFS
//!
//! Provides support for storage, and `PubSub` functionality.

use std::str::FromStr;

/// IPFS Content Identifier.
pub use libipld::Cid;
/// IPLD
pub use libipld::Ipld;
/// libp2p re-export.
pub use rust_ipfs::libp2p::futures::{pin_mut, stream::BoxStream, FutureExt, StreamExt};
/// Peer Info type.
pub use rust_ipfs::p2p::PeerInfo;
/// Enum for specifying paths in IPFS.
pub use rust_ipfs::path::IpfsPath;
/// Server, Client, or Auto mode
pub use rust_ipfs::DhtMode;
/// Server, Client, or Auto mode
pub use rust_ipfs::Ipfs;
/// `PubSub` Message ID type.
pub use rust_ipfs::MessageId;
/// Multiaddr type.
pub use rust_ipfs::Multiaddr;
/// Peer ID type.
pub use rust_ipfs::PeerId;
/// Stream for `PubSub` Topic Subscriptions.
pub use rust_ipfs::SubscriptionStream;
/// Builder type for IPFS Node configuration.
pub use rust_ipfs::UninitializedIpfsNoop as IpfsBuilder;
use rust_ipfs::{dag::ResolveError, unixfs::AddOpt, PubsubEvent, Quorum};

/// Hermes IPFS
#[allow(dead_code)]
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
        // TODO(saibatizoku):
        let node: Ipfs = IpfsBuilder::new()
            .with_default()
            .set_default_listener()
            .start()
            .await?;

        Ok(HermesIpfs { node })
    }

    /// Add a file to IPFS.
    ///
    /// ## Parameters
    ///
    /// * `file_path` The `file_path` can be specified as a type that converts into
    ///   `std::path::PathBuf`.
    ///
    /// ## Returns
    ///
    /// * A result with `IpfsPath`
    ///
    /// ## Errors
    ///
    /// Returns an error if the file fails to upload.
    pub async fn add_ipfs_file(&self, ipfs_file: AddIpfsFile) -> anyhow::Result<IpfsPath> {
        let ipfs_path = self.node.add_unixfs(ipfs_file).await?;
        Ok(ipfs_path)
    }

    /// Get a file from IPFS
    ///
    /// ## Parameters
    ///
    /// * `ipfs_path` - `GetIpfsFile(String)` Path used to get the file from IPFS.
    ///
    /// ## Returns
    ///
    /// * `A result with Vec<u8>`.
    ///
    /// ## Errors
    ///
    /// Returns an error if the file fails to download.
    pub async fn get_ipfs_file(&self, ipfs_path: GetIpfsFile) -> anyhow::Result<Vec<u8>> {
        let stream_bytes = self.node.cat_unixfs(ipfs_path).await?;
        Ok(stream_bytes.to_vec())
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
    pub async fn insert_pin(&self, cid: &Cid) -> anyhow::Result<()> {
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
    pub async fn is_pinned(&self, cid: &Cid) -> anyhow::Result<bool> {
        self.node.is_pinned(cid).await
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
    pub async fn remove_pin(&self, cid: &Cid) -> anyhow::Result<()> {
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
    pub async fn identity(&self, peer_id: Option<PeerId>) -> anyhow::Result<PeerId> {
        self.node.identity(peer_id).await.map(|p| p.peer_id)
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
    pub async fn add_peer(&self, peer_id: PeerId, addr: Multiaddr) -> anyhow::Result<()> {
        self.node.add_peer(peer_id, addr).await
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
    pub async fn dht_mode(&self, mode: DhtMode) -> anyhow::Result<()> {
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
    pub async fn dag_put(&self, ipld: Ipld) -> anyhow::Result<Cid> {
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
    pub async fn dag_get<T: Into<IpfsPath>>(&self, path: T) -> Result<Ipld, ResolveError> {
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
        &self, key: impl AsRef<[u8]>, value: impl Into<Vec<u8>>,
    ) -> anyhow::Result<()> {
        self.node.dht_put(key, value, Quorum::One).await
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
    pub async fn dht_get(&self, key: impl AsRef<[u8]>) -> anyhow::Result<Vec<u8>> {
        let record_stream = self.node.dht_get(key).await?;
        pin_mut!(record_stream);
        let record = record_stream
            .next()
            .await
            .ok_or(anyhow::anyhow!("No record found"))?;
        Ok(record.value)
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
    pub async fn add_bootstrap(&self, address: Multiaddr) -> anyhow::Result<Multiaddr> {
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

    /// Returns a stream of pubsub swarm events for a topic.
    ///
    /// ## Parameters
    ///
    /// * `topic` - `impl Into<String>`
    ///
    /// ## Returns
    ///
    /// * A result with `BoxStream<'static, PubsubEvent>`
    ///
    /// ## Errors
    ///
    /// Returns error if unable to retrieve pubsub swarm events.
    pub async fn pubsub_events(
        &self, topic: impl Into<String>,
    ) -> anyhow::Result<BoxStream<'static, PubsubEvent>> {
        self.node.pubsub_events(topic).await
    }

    /// Subscribes to a pubsub topic.
    ///
    /// ## Parameters
    ///
    /// * `topic` - `impl Into<String>`
    ///
    /// ## Returns
    ///
    /// * `SubscriptionStream`
    ///
    /// ## Errors
    ///
    /// Returns error if unable to subscribe to pubsub topic.
    pub async fn pubsub_subscribe(
        &self, topic: impl Into<String>,
    ) -> anyhow::Result<SubscriptionStream> {
        self.node.pubsub_subscribe(topic).await
    }

    /// Unsubscribes from a pubsub topic.
    ///
    /// ## Parameters
    ///
    /// * `topic` - `impl Into<String>`
    ///
    /// ## Returns
    ///
    /// * `bool`
    ///
    /// ## Errors
    ///
    /// Returns error if unable to unsubscribe from pubsub topic.
    pub async fn pubsub_unsubscribe(&self, topic: impl Into<String>) -> anyhow::Result<bool> {
        self.node.pubsub_unsubscribe(topic).await
    }

    /// Publishes a message to a pubsub topic.
    ///
    /// ## Parameters
    ///
    /// * `topic` - `impl Into<String>`
    /// * `message` - `Vec<u8>`
    ///
    /// ## Returns
    ///
    /// * `Result<MessageId>`
    ///
    /// ## Errors
    ///
    /// Returns error if unable to publish to a pubsub topic.
    pub async fn pubsub_publish(
        &self, topic: impl Into<String>, message: Vec<u8>,
    ) -> anyhow::Result<MessageId> {
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
    pub async fn ban_peer(&self, peer: PeerId) -> anyhow::Result<()> {
        self.node.ban_peer(peer).await
    }
}

impl From<Ipfs> for HermesIpfs {
    fn from(node: Ipfs) -> Self {
        Self { node }
    }
}

/// File that will be added to IPFS
pub enum AddIpfsFile {
    /// Path in local disk storage to the file.
    Path(std::path::PathBuf),
    /// Stream of file bytes, with an optional name.
    /// **NOTE** current implementation of `rust-ipfs` does not add names to published
    /// files.
    Stream((Option<String>, Vec<u8>)),
}

impl From<AddIpfsFile> for AddOpt {
    fn from(value: AddIpfsFile) -> Self {
        match value {
            AddIpfsFile::Path(file_path) => file_path.into(),
            AddIpfsFile::Stream((None, bytes)) => bytes.into(),
            AddIpfsFile::Stream((Some(name), bytes)) => (name, bytes).into(),
        }
    }
}

impl From<String> for AddIpfsFile {
    fn from(value: String) -> Self {
        Self::Path(value.into())
    }
}

impl From<std::path::PathBuf> for AddIpfsFile {
    fn from(value: std::path::PathBuf) -> Self {
        Self::Path(value)
    }
}

impl From<Vec<u8>> for AddIpfsFile {
    fn from(value: Vec<u8>) -> Self {
        Self::Stream((None, value))
    }
}

impl From<(String, Vec<u8>)> for AddIpfsFile {
    fn from((name, stream): (String, Vec<u8>)) -> Self {
        Self::Stream((Some(name), stream))
    }
}

impl From<(Option<String>, Vec<u8>)> for AddIpfsFile {
    fn from(value: (Option<String>, Vec<u8>)) -> Self {
        Self::Stream(value)
    }
}

/// Path to get the file from IPFS
pub struct GetIpfsFile(IpfsPath);

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
