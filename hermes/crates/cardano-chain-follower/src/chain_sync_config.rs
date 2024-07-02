//! Cardano chain sync configuration.
//!
//! Independent of ANY followers, we allow a maximum of 3 Chains being updated, one for
//! each network. Chain Followers use the data supplied by the Chain-Sync.
//! This module configures the chain sync processes.

use crossbeam_skiplist::SkipMap;
use once_cell::sync::Lazy;
use strum::IntoEnumIterator;
use tokio::{sync::Mutex, task::JoinHandle};
use tracing::{debug, error};

use crate::{
    chain_sync::chain_sync,
    error::{Error, Result},
    mithril_snapshot_config::MithrilSnapshotConfig,
    network::Network,
};

/// Default [`Follower`] block buffer size.
const DEFAULT_CHAIN_UPDATE_BUFFER_SIZE: usize = 32;

/// How many slots back from TIP is considered Immutable in the absence of a mithril
/// snapshot.
const DEFAULT_IMMUTABLE_SLOT_WINDOW: u64 = 12 * 60 * 60;

/// Type we use to manage the Sync Task handle map.
type SyncMap = SkipMap<Network, Mutex<Option<JoinHandle<()>>>>;
/// Handle to the mithril sync thread. One for each Network ONLY.
static SYNC_JOIN_HANDLE_MAP: Lazy<SyncMap> = Lazy::new(|| {
    let map = SkipMap::new();
    for network in Network::iter() {
        map.insert(network, Mutex::new(None));
    }
    map
});

/// A Follower Connection to the Cardano Network.
#[derive(Clone, Debug)]
pub struct ChainSyncConfig {
    /// Chain Network
    pub chain: Network,
    /// Relay Node Address
    pub(crate) relay_address: String,
    /// Block buffer size option.
    chain_update_buffer_size: usize,
    /// If we don't have immutable data, how far back from TIP is the data considered
    /// Immutable (in slots).
    immutable_slot_window: u64,
    /// Configuration of Mithril Snapshots.
    pub mithril_cfg: MithrilSnapshotConfig,
}

impl ChainSyncConfig {
    /// Sets the defaults for a given cardano network.
    /// Each network has a different set of defaults, so no single "default" can apply.
    /// This function is preferred to the `default()` standard function.
    #[must_use]
    pub fn default_for(chain: Network) -> Self {
        Self {
            chain,
            relay_address: chain.default_relay(),
            chain_update_buffer_size: DEFAULT_CHAIN_UPDATE_BUFFER_SIZE,
            immutable_slot_window: DEFAULT_IMMUTABLE_SLOT_WINDOW,
            mithril_cfg: MithrilSnapshotConfig::default_for(chain),
        }
    }

    /// Sets the relay to use for Chain Sync.
    ///
    /// # Arguments
    ///
    /// * `relay`: Address to use for the blockchain relay node.
    #[must_use]
    pub fn relay(mut self, address: String) -> Self {
        self.relay_address = address;
        self
    }

    /// Sets the size of the chain updates buffer used by the [`Follower`].
    ///
    /// # Arguments
    ///
    /// * `chain_update_buffer_size`: Size of the chain updates buffer.
    #[must_use]
    pub fn chain_update_buffer_size(mut self, block_buffer_size: usize) -> Self {
        self.chain_update_buffer_size = block_buffer_size;
        self
    }

    /// Sets the size of the Immutable window used when Mithril is not available.
    ///
    /// # Arguments
    ///
    /// * `window`: Size of the Immutable window.
    #[must_use]
    pub fn immutable_slot_window(mut self, window: u64) -> Self {
        self.immutable_slot_window = window;
        self
    }

    /// Sets the the Mithril snapshot Config the `ChainSync` will use.
    ///
    /// # Arguments
    ///
    /// * `path`: Mithril snapshot path.
    /// * `update`: Auto-update this path with the latest mithril snapshot as it changes.
    #[must_use]
    pub fn mithril_cfg(mut self, cfg: MithrilSnapshotConfig) -> Self {
        self.mithril_cfg = cfg;
        self
    }

    /// Runs Chain Synchronization.
    ///
    /// Must be done BEFORE the chain can be followed.
    ///
    /// # Arguments
    ///
    /// * `chain`: The chain to follow.
    ///
    /// # Returns
    ///
    /// `Result<()>`: On success.
    ///
    /// # Errors
    ///
    /// `Error`: On error.
    pub async fn run(self) -> Result<()> {
        debug!(
            chain = self.chain.to_string(),
            "Chain Synchronization Starting"
        );

        // Start the Chain Sync - IFF its not already running.
        let lock_entry = match SYNC_JOIN_HANDLE_MAP.get(&self.chain) {
            None => {
                error!("Join Map improperly initialized: Missing {}!!", self.chain);
                return Err(Error::Internal); // Should not get here.
            },
            Some(entry) => entry,
        };
        let mut locked_handle = lock_entry.value().lock().await;

        if (*locked_handle).is_some() {
            debug!("Chain Sync Already Running for {}", self.chain);
            return Err(Error::ChainSyncAlreadyRunning(self.chain));
        }

        // Start the Mithril Snapshot Follower
        let rx = self.mithril_cfg.run().await?;

        // Start Chain Sync
        *locked_handle = Some(tokio::spawn(chain_sync(self.clone(), rx)));

        // sync_map.insert(chain, handle);
        debug!("Chain Sync for {} : Started", self.chain);

        Ok(())
    }
}
