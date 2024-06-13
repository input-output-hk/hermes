//! Cardano chain follower.

mod follow;
mod mithril_config;
mod mithril_snapshot;
mod mithril_snapshot_downloader;

use std::{io, path::PathBuf, str::FromStr};

pub use follow::*;
pub use pallas::network::miniprotocols::Point;
use pallas::{
    ledger::traverse::{wellknown::GenesisValues, MultiEraBlock},
    network::miniprotocols::{chainsync, MAINNET_MAGIC, PREVIEW_MAGIC, PRE_PRODUCTION_MAGIC},
};
use thiserror::Error;

/// Crate error type.
#[derive(Debug, Error)]
pub enum Error {
    /// Data encoding/decoding error.
    #[error("Codec error: {0:?}")]
    Codec(pallas::ledger::traverse::Error),
    /// Client connection error.
    #[error("Client error: {0:?}")]
    Client(pallas::network::facades::Error),
    /// Blockfetch protocol error.
    #[error("Blockfetch error: {0:?}")]
    Blockfetch(pallas::network::miniprotocols::blockfetch::ClientError),
    /// Chainsync protocol error.
    #[error("Chainsync error: {0:?}")]
    Chainsync(chainsync::ClientError),
    /// Follower failed to set its read pointer.
    #[error("Failed to set follower read pointer")]
    SetReadPointer,
    /// Follower background follow task has stopped.
    #[error("Follower follow task is not running")]
    FollowTaskNotRunning,
    /// Mithril snapshot error.
    #[error("Failed to read block(s) from Mithril snapshot")]
    MithrilSnapshot(Option<pallas_hardano::storage::immutable::Error>),
    /// Mithril snapshot chunk error.
    #[error("Failed to read block(s) from Mithril snapshot")]
    MithrilSnapshotChunk(pallas_hardano::storage::immutable::chunk::Error),
    /// Failed to parse
    #[error("Failed to parse network")]
    ParseNetwork,
    /// Mithril Snapshot path is not a directory
    #[error("Mithril Snapshot path `{0}` is not a directory")]
    MithrilSnapshotDirectoryNotFound(String),
    /// Mithril Snapshot path is already configured for another network
    #[error("Mithril Snapshot path `{0}` is already configured for network `{1}`")]
    MithrilSnapshotDirectoryAlreadyConfiguredForNetwork(PathBuf, Network),
    /// Mithril Snapshot path is already configured for this network
    #[error("Mithril Snapshot path `{0}` is already configured as `{1}`")]
    MithrilSnapshotDirectoryAlreadyConfigured(PathBuf, PathBuf),
    /// Mithril Snapshot path not configured, trying to start auto-update
    #[error("Mithril Snapshot path is not configured.  Can not start Auto Snapshot Update.")]
    MithrilSnapshotDirectoryNotConfigured,
    /// Mithril snapshot directory failed to be created.
    #[error("Mithril Snapshot path `{0}` does not exist, and could not be created. `{1}`")]
    MithrilSnapshotDirectoryCreationError(PathBuf, io::Error),
    /// Mithril snapshot directory is not writable and we need to be able to update the
    /// snapshot data.
    #[error("Mithril Snapshot path `{0}` is not writable, or contains read-only files.")]
    MithrilSnapshotDirectoryNotWritable(PathBuf),
    /// Mithril aggregator URL is already defined for a network.
    #[error("Mithril Aggregator URL `{0}` is already configured as `{1}`")]
    MithrilAggregatorURLAlreadyConfigured(String, String),
    /// Mithril aggregator URL is already defined for a network.
    #[error("Mithril Aggregator URL `{0}` is already configured for network `{1}`")]
    MithrilAggregatorURLAlreadyConfiguredForNetwork(String, Network),
    /// Mithril aggregator URL is not a valid URL
    #[error("Mithril Aggregator URL `{0}` is not a valid URL: `{1}`")]
    MithrilAggregatorURLParseError(String, url::ParseError),
    /// General Mithril Client Error
    #[error("Mithril Client Error for {0} @ {1}: {2}")]
    MithrilClientError(Network, String, anyhow::Error),
    /// Mithril Aggregator has no Snapshots
    #[error("Mithril Aggregator does not list any Mithril Snapshots for {0} @ {1}")]
    MithrilClientNoSnapshotsError(Network, String),
    /// Mithril Aggregator mismatch
    #[error("Mithril Aggregator network mismatch.  Wanted {0} Got {1}")]
    MithrilClientNetworkMismatch(Network, String),
    /// Mithril genesis VKEY Mismatch
    #[error("Mithril Genesis VKEY for Network {0} is already set, and can not be changed to a different value.")]
    MithrilGenesisVKeyMismatch(Network),
    /// Mithril genesis VKEY is not properly HEX Encoded
    #[error("Mithril Genesis VKEY for Network {0} is not hex encoded.  Needs to be only HEX Ascii characters, and even length.")]
    MithrilGenesisVKeyNotHex(Network),
    /// Mithril Autoupdate requires an Aggregator and a VKEY and a Path
    #[error("Mithril Auto Update Network {0} failed to start. No Aggregator and/or Genesis VKEY and/or Path are configured.")]
    MithrilUpdateRequiresAggregatorAndVkeyAndPath(Network),
    /// Internal Error
    #[error("Internal error")]
    InternalError,
}

/// Crate result type.
pub type Result<T> = std::result::Result<T, Error>;

/// A point in the chain or the tip.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum PointOrTip {
    /// Represents a specific point of the chain.
    Point(Point),
    /// Represents the tip of the chain.
    Tip,
}

impl From<Point> for PointOrTip {
    fn from(point: Point) -> Self {
        Self::Point(point)
    }
}

/// CBOR encoded data of a multi-era block.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MultiEraBlockData(Vec<u8>);

impl MultiEraBlockData {
    /// Decodes the data into a multi-era block.
    ///
    /// # Errors
    ///
    /// Returns Err if the block's era couldn't be decided or if the encoded data is
    /// invalid.
    pub fn decode(&self) -> Result<MultiEraBlock> {
        let block = MultiEraBlock::decode(&self.0).map_err(Error::Codec)?;

        Ok(block)
    }

    /// Consumes the [`MultiEraBlockData`] returning the block data raw bytes.
    #[must_use]
    pub fn into_raw_data(self) -> Vec<u8> {
        self.0
    }
}

impl AsRef<[u8]> for MultiEraBlockData {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Enum of possible Cardano networks.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Network {
    /// Cardano mainnet network.
    Mainnet,
    /// Cardano pre-production network.
    Preprod,
    /// Cardano preview network.
    Preview,
}

/// The human readable name of the Cardano mainnet network.
const MAINNET_NAME: &str = "mainnet";
/// The human readable name of the Cardano pre-production network.
const PREPROD_NAME: &str = "preprod";
/// The human readable name of the Cardano preview network.
const PREVIEW_NAME: &str = "preview";

impl FromStr for Network {
    type Err = Error;

    fn from_str(input: &str) -> std::result::Result<Network, Self::Err> {
        match input {
            MAINNET_NAME => Ok(Network::Mainnet),
            PREPROD_NAME => Ok(Network::Preprod),
            PREVIEW_NAME => Ok(Network::Preview),
            _ => Err(Error::ParseNetwork),
        }
    }
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Network::Mainnet => write!(f, "{MAINNET_NAME}"),
            Network::Preprod => write!(f, "{PREPROD_NAME}"),
            Network::Preview => write!(f, "{PREVIEW_NAME}"),
        }
    }
}

impl From<Network> for u64 {
    fn from(network: Network) -> Self {
        match network {
            Network::Mainnet => MAINNET_MAGIC,
            Network::Preprod => PRE_PRODUCTION_MAGIC,
            Network::Preview => PREVIEW_MAGIC,
        }
    }
}

/// Return genesis values for given network
#[must_use]
pub fn network_genesis_values(network: &Network) -> Option<GenesisValues> {
    match network {
        Network::Mainnet => GenesisValues::from_magic(MAINNET_MAGIC),
        Network::Preprod => GenesisValues::from_magic(PRE_PRODUCTION_MAGIC),
        Network::Preview => GenesisValues::from_magic(PREVIEW_MAGIC),
    }
}

/// Validate a multi-era block.
///
/// This does not execute Plutus scripts nor validates ledger state.
/// It only checks that the block is correctly formatted for its era.
#[allow(dead_code)]
fn validate_multiera_block(_block: &MultiEraBlock) {
    // (fsgr): Not sure about hwo the validation will be done in here yet.
    todo!()
}
