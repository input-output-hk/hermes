//! Cardano chain follower.

mod follow;
mod mithril_snapshot;

use std::str::FromStr;

pub use follow::*;
pub use pallas::network::miniprotocols::Point;
use pallas::{
    ledger::traverse::{wellknown::GenesisValues, MultiEraBlock},
    network::miniprotocols::{
        chainsync, MAINNET_MAGIC, PREVIEW_MAGIC, PRE_PRODUCTION_MAGIC, TESTNET_MAGIC,
    },
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
    MithrilSnapshot,
    /// Failed to parse
    #[error("Failed to parse network")]
    ParseNetwork,
    /// Internal Error
    #[error("Internal error")]
    InternalError,
}

/// Crate result type.
pub type Result<T> = std::result::Result<T, Error>;

/// A point in the chain or the tip.
#[derive(Clone, PartialEq, Eq, Hash)]
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
#[derive(Clone, PartialEq, Eq, Hash)]
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
    /// Cardano testnet network.
    Testnet,
}

/// The human readable name of the Cardano mainnet network.
const MAINNET_NAME: &str = "mainnet";
/// The human readable name of the Cardano pre-production network.
const PREPROD_NAME: &str = "preprod";
/// The human readable name of the Cardano preview network.
const PREVIEW_NAME: &str = "preview";
/// The human readable name of a Cardano local testnet network.
const TESTNET_NAME: &str = "testnet";

impl FromStr for Network {
    type Err = Error;

    fn from_str(input: &str) -> std::result::Result<Network, Self::Err> {
        match input {
            MAINNET_NAME => Ok(Network::Mainnet),
            PREPROD_NAME => Ok(Network::Preprod),
            PREVIEW_NAME => Ok(Network::Preview),
            TESTNET_NAME => Ok(Network::Testnet),
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
            Network::Testnet => write!(f, "{TESTNET_NAME}"),
        }
    }
}

impl From<Network> for u64 {
    fn from(network: Network) -> Self {
        match network {
            Network::Mainnet => MAINNET_MAGIC,
            Network::Preprod => PRE_PRODUCTION_MAGIC,
            Network::Preview => PREVIEW_MAGIC,
            Network::Testnet => TESTNET_MAGIC,
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
        Network::Testnet => GenesisValues::from_magic(TESTNET_MAGIC),
    }
}

/// Validate a multi-era block.
///
/// This does not execute Plutus scripts nor validates ledger state.
/// It only checks that the block is correctly formatted for its era.
#[allow(dead_code)]
fn validate_multiera_block(_block: &MultiEraBlock) {
    // (fsgr): Not sure about hwo the validation will be done in here yet.
}
