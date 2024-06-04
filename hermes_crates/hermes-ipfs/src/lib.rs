//! Hermes IPFS
//!
//! Provides support for storage, and `PubSub` functionality.

use std::str::FromStr;

use rust_ipfs::{unixfs::AddOpt, Ipfs, UninitializedIpfsNoop};

/// IPFS Content Identifier.
pub use libipld::Cid;
/// Enum for specifying paths in IPFS.
pub use rust_ipfs::path::IpfsPath;

/// Hermes IPFS Node
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
        let node = UninitializedIpfsNoop::new().with_default().start().await?;
        Ok(HermesIpfs { node })
    }

    /// Add a file to IPFS.
    ///
    /// ## Parameters
    ///
    /// * `file_path` The `file_path` can be specified as a type that converts into `std::path::PathBuf`.
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
        let ipfs_path: IpfsPath = ipfs_path.try_into()?;
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
    /// Works correctly only under no-crash situations. Workaround for hitting a crash is to re-pin
    /// any existing recursive pins.
    ///
    /// ## Parameters
    ///
    /// * `cid` - `Cid` Content identifier to be pinned.
    ///
    /// ## Returns
    /// `true` if the block is pinned, `false` if not. See Crash unsafety notes for the false
    /// response.
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
}

/// File that will be added to IPFS
pub enum AddIpfsFile {
    /// Path in local disk storage to the file.
    Path(std::path::PathBuf),
    /// Stream of file bytes, with an optional name.
    /// **NOTE** current implementation of `rust-ipfs` does not add names to published files.
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

/// Path to get the file from IPFS
pub struct GetIpfsFile(pub String);

impl TryFrom<GetIpfsFile> for IpfsPath {
    type Error = anyhow::Error;

    fn try_from(GetIpfsFile(ipfs_path): GetIpfsFile) -> Result<Self, Self::Error> {
        IpfsPath::from_str(&ipfs_path)
    }
}