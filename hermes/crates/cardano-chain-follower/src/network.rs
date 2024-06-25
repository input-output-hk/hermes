//! Enum of possible Cardano networks.

use std::{ffi::OsStr, path::PathBuf, str::FromStr};

use crate::error::Error;

use pallas::{
    ledger::traverse::wellknown::GenesisValues,
    network::miniprotocols::{MAINNET_MAGIC, PREVIEW_MAGIC, PRE_PRODUCTION_MAGIC},
};
//use strum::IntoEnumIterator;
//use strum_macros;
use tracing::debug;

/// Default name of the executable if we can't derive it.
pub(crate) const DEFAULT_EXE_NAME: &str = "cardano_chain_follower";
/// ENV VAR name for the data path.
pub(crate) const ENVVAR_MITHRIL_DATA_PATH: &str = "MITHRIL_DATA_PATH";
/// ENV VAR name for the executable name.
pub(crate) const ENVVAR_MITHRIL_EXE_NAME: &str = "MITHRIL_EXE_NAME";

/// Enum of possible Cardano networks.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, strum::EnumIter)]
pub enum Network {
    /// Cardano mainnet network.
    Mainnet,
    /// Cardano pre-production network.
    Preprod,
    /// Cardano preview network.
    Preview,
}

// Mainnet Defaults.
/// The human readable name of the Cardano mainnet network.
const MAINNET_NAME: &str = "mainnet";
/// Mainnet Default Public Cardano Relay.
const DEFAULT_MAINNET_RELAY: &str = "backbone.cardano.iog.io:3001";
/// Main-net Mithril Signature genesis vkey.
const DEFAULT_MAINNET_MITHRIL_GENESIS_KEY: &str = include_str!("data/mainnet-genesis.vkey");
/// Default Mithril Aggregator to use.
const DEFAULT_MAINNET_MITHRIL_AGGREGATOR: &str =
    "https://aggregator.release-mainnet.api.mithril.network/aggregator";

// Preprod Defaults
/// The human readable name of the Cardano pre-production network.
const PREPROD_NAME: &str = "preprod";
/// Preprod Default Public Cardano Relay.
const DEFAULT_PREPROD_RELAY: &str = "preprod-node.play.dev.cardano.org:3001";
/// Preprod network Mithril Signature genesis vkey.
const DEFAULT_PREPROD_MITHRIL_GENESIS_KEY: &str = include_str!("data/preprod-genesis.vkey");
/// Default Mithril Aggregator to use.
const DEFAULT_PREPROD_MITHRIL_AGGREGATOR: &str =
    "https://aggregator.release-preprod.api.mithril.network/aggregator";

// Preview Defaults
/// The human readable name of the Cardano preview network.
const PREVIEW_NAME: &str = "preview";
/// Preview Default Public Cardano Relay.
const DEFAULT_PREVIEW_RELAY: &str = "preview-node.play.dev.cardano.org:3001";
/// Preview network Mithril Signature genesis vkey.
const DEFAULT_PREVIEW_MITHRIL_GENESIS_KEY: &str = include_str!("data/preview-genesis.vkey");
/// Default Mithril Aggregator to use.
const DEFAULT_PREVIEW_MITHRIL_AGGREGATOR: &str =
    "https://aggregator.pre-release-preview.api.mithril.network/aggregator";

impl Network {
    /// Get the default Relay for a blockchain network.
    #[must_use]
    pub fn default_relay(self) -> String {
        match self {
            Network::Mainnet => DEFAULT_MAINNET_RELAY.to_string(),
            Network::Preprod => DEFAULT_PREPROD_RELAY.to_string(),
            Network::Preview => DEFAULT_PREVIEW_RELAY.to_string(),
        }
    }

    /// Get the default aggregator for a blockchain.
    #[must_use]
    pub fn default_mithril_aggregator(self) -> String {
        match self {
            Network::Mainnet => DEFAULT_MAINNET_MITHRIL_AGGREGATOR.to_string(),
            Network::Preprod => DEFAULT_PREPROD_MITHRIL_AGGREGATOR.to_string(),
            Network::Preview => DEFAULT_PREVIEW_MITHRIL_AGGREGATOR.to_string(),
        }
    }

    /// Get the default Mithril Signature genesis key for a blockchain.
    #[must_use]
    pub fn default_mithril_genesis_key(self) -> String {
        match self {
            Network::Mainnet => DEFAULT_MAINNET_MITHRIL_GENESIS_KEY.to_string(),
            Network::Preprod => DEFAULT_PREPROD_MITHRIL_GENESIS_KEY.to_string(),
            Network::Preview => DEFAULT_PREVIEW_MITHRIL_GENESIS_KEY.to_string(),
        }
    }

    /// Get the default storage location for mithril snapshots.
    /// Defaults to: <platform data_local_dir>/<exe name>/mithril/<network>
    pub fn default_mithril_path(self) -> PathBuf {
        // Get the base path for storing Data.
        // IF the ENV var is set, use that.
        // Otherwise use the system default data path for an application.
        // All else fails default to "/var/lib"
        let mut base_path = std::env::var(ENVVAR_MITHRIL_DATA_PATH).map_or_else(
            |_| dirs::data_local_dir().unwrap_or("/var/lib".into()),
            PathBuf::from,
        );

        // Get the Executable name for the data path.
        // IF the ENV var is set, use it, otherwise try and get it from the exe itself.
        // Fallback to using a default exe name if all else fails.
        let exe_name = std::env::var(ENVVAR_MITHRIL_EXE_NAME).unwrap_or(
            std::env::current_exe()
                .unwrap_or(DEFAULT_EXE_NAME.into())
                .file_name()
                .unwrap_or(OsStr::new(DEFAULT_EXE_NAME))
                .to_string_lossy()
                .to_string(),
        );

        // <base path>/<exe name>
        base_path.push(exe_name);

        // Put everything in a `mithril` sub directory.
        base_path.push("mithril");

        // <base path>/<exe name>/<network>
        base_path.push(self.to_string());

        debug!(
            chain = self.to_string(),
            path = base_path.to_string_lossy().to_string(),
            "DEFAULT Mithril Data Path",
        );

        // Return the final path
        base_path
    }

    /// Return genesis values for given network
    #[must_use]
    pub fn genesis_values(self) -> Option<GenesisValues> {
        match self {
            Network::Mainnet => GenesisValues::from_magic(MAINNET_MAGIC),
            Network::Preprod => GenesisValues::from_magic(PRE_PRODUCTION_MAGIC),
            Network::Preview => GenesisValues::from_magic(PREVIEW_MAGIC),
        }
    }
}

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
