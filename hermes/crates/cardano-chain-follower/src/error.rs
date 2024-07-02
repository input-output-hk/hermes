//! Library Crates Defined Errors

use std::{io, path::PathBuf};

use pallas::network::miniprotocols::chainsync;
use thiserror::Error;

use crate::network::Network;

/// Crate error type.
#[derive(Debug, Error)]
pub enum Error {
    /// Data encoding/decoding error.
    #[error("Codec error: {0:?}")]
    Codec(String),
    /// Client connection error.
    #[error("Client error: {0:?}")]
    Client(pallas::network::facades::Error),
    /// Blockfetch protocol error.
    #[error("Blockfetch error: {0:?}")]
    Blockfetch(pallas::network::miniprotocols::blockfetch::ClientError),
    /// Chainsync protocol error.
    #[error("Chainsync error: {0:?}")]
    Chainsync(chainsync::ClientError),
    /// Live Sync error.
    #[error("Live Sync error: {0:?}")]
    LiveSync(String),
    /// Follower failed to set its read pointer.
    #[error("Failed to set follower read pointer")]
    SetReadPointer,
    /// Follower background follow task has stopped.
    #[error("Follower follow task is not running")]
    FollowTaskNotRunning,
    /// Chain Sync already running error.
    #[error("Chain Sync already running for network: {0}")]
    ChainSyncAlreadyRunning(Network),
    /// Mithril snapshot already running error.
    #[error("Mithril Snapshot Sync already running for network: {0}")]
    MithrilSnapshotSyncAlreadyRunning(Network),
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
    MithrilSnapshotDirectoryCreation(PathBuf, io::Error),
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
    MithrilAggregatorURLParse(String, url::ParseError),
    /// General Mithril Client Error
    #[error("Mithril Client Error for {0} @ {1}: {2}")]
    MithrilClient(Network, String, anyhow::Error),
    /// Mithril Aggregator has no Snapshots
    #[error("Mithril Aggregator does not list any Mithril Snapshots for {0} @ {1}")]
    MithrilClientNoSnapshots(Network, String),
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
    Internal,
}

/// Crate result type.
pub type Result<T> = std::result::Result<T, Error>;
