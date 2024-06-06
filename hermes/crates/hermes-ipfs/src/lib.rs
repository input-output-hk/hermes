//! Hermes IPFS
//!
//! Provides support for storage, and `PubSub` functionality.

use std::{
    ops::{Deref, DerefMut},
    str::FromStr,
};

/// IPFS Content Identifier.
pub use libipld::Cid;
/// Peer Info type.
pub use rust_ipfs::p2p::PeerInfo;
/// Enum for specifying paths in IPFS.
pub use rust_ipfs::path::IpfsPath;
/// Multiaddr type.
pub use rust_ipfs::Multiaddr;
/// Peer ID type.
pub use rust_ipfs::PeerId;
use rust_ipfs::{unixfs::AddOpt, Ipfs, UninitializedIpfsNoop};

/// Hermes IPFS Node
///
/// Provides the functionality of the inner `IPFS` by de-referencing.
#[derive(Clone, Debug)]
pub struct Node(Ipfs);

impl From<Ipfs> for Node {
    fn from(value: Ipfs) -> Self {
        Self(value)
    }
}

impl Deref for Node {
    type Target = Ipfs;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Node {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Hermes IPFS
#[allow(dead_code)]
pub struct HermesIpfs {
    /// IPFS node
    node: Node,
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
        let node = UninitializedIpfsNoop::new()
            .with_default()
            .set_default_listener()
            .start()
            .await?
            .into();
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
        self.node.0.exit_daemon().await;
    }

    /// Returns the peer identity information. If no peer id is supplied the local node
    /// identity is used.
    ///
    /// ## Errors
    ///
    /// Returns error if peer info cannot be retrieved.
    pub async fn identity(&self, peer_id: Option<PeerId>) -> anyhow::Result<PeerInfo> {
        self.node.identity(peer_id).await
    }

    /// Add peer to address book
    ///
    /// ## Errors
    ///
    /// Returns error if unable to add peer.
    pub async fn add_peer(&self, peer_id: PeerId, addr: Multiaddr) -> anyhow::Result<()> {
        self.node.add_peer(peer_id, addr).await
    }

    /// Returns local listening addresses
    ///
    /// ## Errors
    ///
    /// Returns error if listening addresses cannot be retrieved.
    pub async fn listening_addresses(&self) -> anyhow::Result<Vec<Multiaddr>> {
        self.node.listening_addresses().await
    }

    #[must_use]
    /// Gets the inner node for direct manipulation.
    pub fn node(&self) -> Node {
        self.node.clone()
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
