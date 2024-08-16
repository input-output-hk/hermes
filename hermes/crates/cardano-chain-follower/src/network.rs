//! Enum of possible Cardano networks.

use std::{ffi::OsStr, path::PathBuf};

use chrono::{DateTime, Utc};
use pallas::{
    ledger::traverse::wellknown::GenesisValues,
    network::miniprotocols::{MAINNET_MAGIC, PREVIEW_MAGIC, PRE_PRODUCTION_MAGIC},
};
// use strum::IntoEnumIterator;
// use strum_macros;
use tracing::debug;

/// Default name of the executable if we can't derive it.
pub(crate) const DEFAULT_EXE_NAME: &str = "cardano_chain_follower";
/// ENV VAR name for the data path.
pub(crate) const ENVVAR_MITHRIL_DATA_PATH: &str = "MITHRIL_DATA_PATH";
/// ENV VAR name for the executable name.
pub(crate) const ENVVAR_MITHRIL_EXE_NAME: &str = "MITHRIL_EXE_NAME";

/// Enum of possible Cardano networks.
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    strum::EnumIter,
    strum::VariantNames,
    strum::EnumString,
    strum::Display,
)]
#[strum(ascii_case_insensitive)]
pub enum Network {
    /// Cardano mainnet network.
    Mainnet,
    /// Cardano pre-production network.
    Preprod,
    /// Cardano preview network.
    Preview,
}

// Mainnet Defaults.
/// Mainnet Default Public Cardano Relay.
const DEFAULT_MAINNET_RELAY: &str = "backbone.cardano.iog.io:3001";
/// Main-net Mithril Signature genesis vkey.
const DEFAULT_MAINNET_MITHRIL_GENESIS_KEY: &str = include_str!("data/mainnet-genesis.vkey");
/// Default Mithril Aggregator to use.
const DEFAULT_MAINNET_MITHRIL_AGGREGATOR: &str =
    "https://aggregator.release-mainnet.api.mithril.network/aggregator";

// Preprod Defaults
/// Preprod Default Public Cardano Relay.
const DEFAULT_PREPROD_RELAY: &str = "preprod-node.play.dev.cardano.org:3001";
/// Preprod network Mithril Signature genesis vkey.
const DEFAULT_PREPROD_MITHRIL_GENESIS_KEY: &str = include_str!("data/preprod-genesis.vkey");
/// Default Mithril Aggregator to use.
const DEFAULT_PREPROD_MITHRIL_AGGREGATOR: &str =
    "https://aggregator.release-preprod.api.mithril.network/aggregator";

// Preview Defaults
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
    pub fn genesis_values(self) -> GenesisValues {
        match self {
            Network::Mainnet => GenesisValues::mainnet(),
            Network::Preprod => GenesisValues::preprod(),
            Network::Preview => GenesisValues::preview(),
        }
    }

    /// Convert a given slot# to its Wall Time for a Blockchain network.
    #[must_use]
    pub fn slot_to_time(&self, slot: u64) -> DateTime<Utc> {
        let genesis = self.genesis_values();
        let wall_clock = genesis.slot_to_wallclock(slot);

        let raw_time: i64 = wall_clock.try_into().unwrap_or(i64::MAX);
        DateTime::from_timestamp(raw_time, 0).unwrap_or(DateTime::<Utc>::MAX_UTC)
    }

    /// Convert an arbitrary time to a slot.
    ///
    /// If the given time predates the blockchain, will return None.
    ///
    /// The Slot does not have to be a valid slot present in the blockchain.
    #[must_use]
    pub fn time_to_slot(&self, _time: DateTime<Utc>) -> Option<u64> {
        // TODO: Implement this, for now just return None.
        None
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use anyhow::Ok;

    use super::*;

    #[test]
    fn test_from_str() -> anyhow::Result<()> {
        let mainnet = Network::from_str("mainnet")?;
        let preprod = Network::from_str("preprod")?;
        let preview = Network::from_str("preview")?;

        assert_eq!(mainnet, Network::Mainnet);
        assert_eq!(preprod, Network::Preprod);
        assert_eq!(preview, Network::Preview);

        let mainnet = Network::from_str("Mainnet")?;
        let preprod = Network::from_str("Preprod")?;
        let preview = Network::from_str("Preview")?;

        assert_eq!(mainnet, Network::Mainnet);
        assert_eq!(preprod, Network::Preprod);
        assert_eq!(preview, Network::Preview);

        Ok(())
    }
}
