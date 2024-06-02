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
    /// # Errors
    ///
    /// Returns an error if the IPFS daemon fails to start.
    pub async fn new() -> anyhow::Result<Self> {
        // TODO(saibatizoku):
        let node = UninitializedIpfsNoop::new().with_default().start().await?;
        Ok(HermesIpfs { node })
    }

    /// Add a file to IPFS.
    ///
    /// # Parameters
    ///
    /// * `payload` The payload can be specified as a file path, as a stream of bytes, or as a
    ///     named stream of bytes.
    ///
    ///     * File path types
    ///         * `&str`
    ///         * `String`
    ///         * `&std::path::Path`
    ///         * `std::path::PathBuf`
    ///
    ///     * Stream of bytes
    ///         * `Vec<u8>`
    ///         * `&static [u8]`
    ///
    ///     * Named stream of bytes
    ///         * `(String, Vec<u8>)`
    ///         * `(String, &static [u8])`
    ///
    /// # Errors
    ///
    /// Returns an error if the file fails to upload.
    pub async fn add_file(&self, payload: impl Into<AddOpt>) -> anyhow::Result<IpfsPath> {
        let mut stream = self.node.add_unixfs(payload);
        let mut ipfs_path = None;

        while let Some(status) = stream.next().await {
            match status {
                UnixfsStatus::ProgressStatus {
                    written,
                    total_size,
                } => match total_size {
                    Some(size) => println!("{written} out of {size} stored"),
                    None => println!("{written} been stored"),
                },
                UnixfsStatus::FailedStatus {
                    written,
                    total_size,
                    error,
                } => {
                    match total_size {
                        Some(size) => println!("failed with {written} out of {size} stored"),
                        None => println!("failed with {written} stored"),
                    }

                    if let Some(error) = error {
                        anyhow::bail!(error);
                    }
                    return Err(Error::AddFileFailure.into());
                }
                UnixfsStatus::CompletedStatus { path, written, .. } => {
                    println!("{written} bytes stored with path {path}");
                    ipfs_path = Some(path);
                }
            }
        }
        ipfs_path.ok_or(Error::AddFileFailure.into())
    }
}

/// Hermes IPFS Errors.
#[derive(thiserror::Error, Debug)]
enum Error {
    /// File upload to IPFS failed.
    #[error = "failed to add file to ipfs"]
    AddFileFailure,
}
