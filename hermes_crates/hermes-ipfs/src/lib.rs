//! Hermes IPFS
//!
//! Provides support for storage, and `PubSub` functionality.

use futures::StreamExt;

use rust_ipfs::{unixfs::UnixfsStatus, Ipfs, UninitializedIpfsNoop};

pub use rust_ipfs::{path::IpfsPath, unixfs::AddOpt};

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
    pub async fn new() -> anyhow::Result<Self> {
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
    ///     ** For example:**
    ///         * `&str`
    ///         * `String`
    ///         * `std::path::PathBuf`
    ///
    /// ## Returns
    ///
    /// * `IpfsPath`
    ///
    /// ## Errors
    ///
    /// Returns an error if the file fails to upload.
    pub async fn add_ipfs_file(
        &self,
        file_path: impl Into<std::path::PathBuf>,
    ) -> anyhow::Result<IpfsPath> {
        let ipfs_path = self.node.add_unixfs(file_path.into()).await?;
        Ok(ipfs_path)
    }

    /// Get a file from IPFS
    ///
    /// ## Parameters
    ///
    /// * `file_path` The `file_path` can be specified as a type that converts into `IpfsPath`.
    ///
    ///     ** For example:**
    ///         * `&str`
    ///         * `String`
    ///
    /// ## Returns
    ///
    /// * `Vec<u8>`
    ///
    /// ## Errors
    ///
    /// Returns an error if the file fails to download.
    pub async fn get_ipfs_file<T: Into<IpfsPath>>(&self, file_path: T) -> anyhow::Result<Vec<u8>> {
        let stream_bytes = self.node.cat_unixfs(file_path).await?;
        Ok(stream_bytes.to_vec())
    }
}

/// Hermes IPFS Errors.
#[derive(thiserror::Error, Debug)]
enum Error {
    /// File upload to IPFS failed.
    #[error = "failed to add file to ipfs"]
    AddFileFailure,
}
