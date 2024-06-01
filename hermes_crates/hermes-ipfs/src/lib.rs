//! Hermes IPFS
//!
//! Provides support for storage, and `PubSub` functionality.
use std::ops::{Deref, DerefMut};

use rust_ipfs::{Ipfs, UninitializedIpfsNoop};

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
        let node = UninitializedIpfsNoop::new()
            .with_default()
            .with_mdns()
            .with_relay(true)
            .default_record_key_validator()
            .start()
            .await?;
        Ok(HermesIpfs { node })
    }
}

impl Deref for HermesIpfs {
    type Target = Ipfs;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl DerefMut for HermesIpfs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node
    }
}
