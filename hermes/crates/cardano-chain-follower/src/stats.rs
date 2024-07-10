//! Cardano Chain Follower Statistics

use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use crossbeam_skiplist::SkipMap;
use once_cell::sync::Lazy;
use serde::Serialize;
use strum::{EnumIter, IntoEnumIterator};
use tracing::error;

use crate::Network;

// -------- GENERAL STATISTIC TRACKING

/// Statistics related to Mithril Snapshots
#[derive(Debug, Default, Clone, Serialize)]
pub struct Mithril {
    /// Number of Mithril Snapshots that have downloaded successfully.
    pub updates: u64,
    /// The Immutable TIP Slot# - Origin = No downloaded snapshot
    pub tip: u64,
    /// Time we started downloading the current snapshot. 1/1/1970-00:00:00 UTC = Never
    /// downloaded.
    pub dl_start: DateTime<Utc>,
    /// Time we finished downloading the current snapshot. if < `dl_start` its the
    /// previous time we finished.
    pub dl_end: DateTime<Utc>,
    /// Number of times download failed (bad server connection)
    pub dl_failures: u64,
    /// The time the last download took, in seconds.
    pub last_dl_duration: u64,
    /// The size of the download archive, in bytes. (If not started and not ended, current
    /// partial download size).
    pub dl_size: u64,
    /// Extraction start time. 1/1/1970-00:00:00 UTC = Never extracted.
    pub extract_start: DateTime<Utc>,
    /// Extraction end time. if `extract_end` < `extract_start` its the previous time we
    /// finished extracting.
    pub extract_end: DateTime<Utc>,
    /// Number of times extraction failed (bad archive)
    pub extract_failures: u64,
    /// Size of last extracted snapshot, in bytes.
    pub extract_size: u64,
    /// Deduplicated Size vs previous snapshot.
    pub deduplicated_size: u64,
    /// Number of identical files deduplicated from previous snapshot.
    pub deduplicated: u64,
    /// Number of changed files from previous snapshot.
    pub changed: u64,
    /// Number of new files from previous snapshot.
    pub new: u64,
    /// Mithril Certificate Validation Start Time. 1/1/1970-00:00:00 UTC = Never
    /// validated.
    pub validate_start: DateTime<Utc>,
    /// Mithril Certificate Validation End Time. if validate end < validate start its the
    /// previous time we finished validating.
    pub validate_end: DateTime<Utc>,
    /// Number of times validation failed (bad snapshot)
    pub validate_failures: u64,
    /// Blocks that failed to deserialize from the mithril immutable chain.
    pub invalid_blocks: u64,
}

impl Mithril {
    /// Reset incremental counters in the mithril statistics.
    fn reset(&mut self) {
        self.updates = 0;
        self.dl_failures = 0;
        self.extract_failures = 0;
        self.validate_failures = 0;
        self.invalid_blocks = 0;
    }
}

/// Statistics related to a single depth of rollback
#[derive(Debug, Default, Clone, Serialize)]
pub struct Rollback {
    /// How deep was the rollback from tip.
    pub depth: u64,
    /// How many times has a rollback been this deep.
    pub count: u64,
}

/// Statistics for all our known rollback types
/// Rollback Vec is sorted by depth, ascending.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Rollbacks {
    /// These are the ACTUAL rollbacks we did on our live-chain in memory.
    pub live: Vec<Rollback>,
    /// These are the rollbacks reported by the Peer Node, which may not == an actual
    /// rollback on our internal live chain.
    pub peer: Vec<Rollback>,
    /// These are the rollbacks synthesized for followers, based on their reading of the
    /// chain tip.
    pub follower: Vec<Rollback>,
}

/// Individual Follower stats
#[derive(Debug, Default, Clone, Serialize)]
pub struct Follower {
    /// Synthetic follower connection ID
    pub id: u64,
    /// Starting slot for this follower (0 = Start at Genesis Block for the chain).
    pub start: u64,
    /// Current slot for this follower.
    pub current: u64,
    /// Target slot for this follower (MAX U64 == Follow Tip Forever).
    pub end: u64,
    /// Current Sync Time.
    pub sync_start: DateTime<Utc>,
    /// When this follower reached TIP or its destination slot.
    pub sync_end: Option<DateTime<Utc>>,
}

/// Statistics related to the live blockchain
#[derive(Debug, Default, Clone, Serialize)]
pub struct Live {
    /// The Time that synchronization to this blockchain started
    pub sync_start: DateTime<Utc>,
    /// The Time that synchronization to this blockchain was complete up-to-tip. None =
    /// Not yet synchronized.
    pub sync_end: Option<DateTime<Utc>>,
    /// When backfill started
    pub backfill_start: Option<DateTime<Utc>>,
    /// Backfill size to achieve synchronization. (0 before sync completed)
    pub backfill_size: u64,
    /// When backfill ended
    pub backfill_end: Option<DateTime<Utc>>,
    /// Backfill Failures
    pub backfill_failures: u64,
    /// The time of the last backfill failure
    pub backfill_failure_time: Option<DateTime<Utc>>,
    /// Current Number of Live Blocks
    pub blocks: u64,
    /// The current head of the live chain slot#
    pub head_slot: u64,
    /// The current live tip slot# as reported by the peer.
    pub tip: u64,
    /// Number of times we connected/re-connected to the Node.
    pub reconnects: u64,
    /// Last reconnect time,
    pub last_connect: DateTime<Utc>,
    /// Last reconnect time,
    pub last_connected_peer: String,
    /// Last disconnect time,
    pub last_disconnect: DateTime<Utc>,
    /// Last disconnect time,
    pub last_disconnected_peer: String,
    /// Is there an active connection to the node
    pub connected: bool,
    /// Rollback statistics.
    pub rollbacks: Rollbacks,
    /// New blocks read from blockchain.
    pub new_blocks: u64,
    /// Blocks that failed to deserialize from the blockchain.
    pub invalid_blocks: u64,
    /// Active Followers (range and current depth)
    pub follower: Vec<Follower>,
}

impl Live {
    /// Reset incremental counters in the live statistics.
    fn reset(&mut self) {
        self.new_blocks = 0;
        self.reconnects = 0;
        self.invalid_blocks = 0;
    }
}

/// Statistics for a single follower network.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Statistics {
    /// Statistics related to the live connection to the blockchain.
    pub live: Live,
    /// Statistics related to the mithril certified blockchain archive.
    pub mithril: Mithril,
}

/// Type we use to manage the Sync Task handle map.
type StatsMap = SkipMap<Network, Arc<RwLock<Statistics>>>;
/// The statistics being maintained per chain.
static STATS_MAP: Lazy<StatsMap> = Lazy::new(|| {
    let map = StatsMap::default();

    for network in Network::iter() {
        let stats = Statistics::default();
        map.insert(network, Arc::new(RwLock::new(stats)));
    }
    map
});

/// Get the stats for a particular chain.
fn lookup_stats(chain: Network) -> Option<Arc<RwLock<Statistics>>> {
    let Some(chain_entry) = STATS_MAP.get(&chain) else {
        error!("Stats MUST BE exhaustively pre-allocated.");
        return None;
    };

    let chain_stats = chain_entry.value();

    Some(chain_stats.clone())
}

impl Statistics {
    /// Get a new statistics struct for a given blockchain network.
    #[must_use]
    pub fn new(chain: Network) -> Self {
        let Some(stats) = lookup_stats(chain) else {
            return Statistics::default();
        };

        let Ok(chain_stats) = stats.read() else {
            return Statistics::default();
        };

        let mut this_stats = chain_stats.clone();
        // Set the current rollback stats.
        this_stats.live.rollbacks.live = rollbacks(chain, RollbackType::LiveChain);
        this_stats.live.rollbacks.peer = rollbacks(chain, RollbackType::Peer);
        this_stats.live.rollbacks.follower = rollbacks(chain, RollbackType::Follower);

        this_stats
    }

    /// Reset the incremental counters in a stats record.
    fn reset_stats(&mut self) {
        self.live.reset();
        self.mithril.reset();
    }

    /// Reset amd return cumulative counters contained in the statistics.
    #[must_use]
    pub fn reset(chain: Network) -> Self {
        let Some(stats) = lookup_stats(chain) else {
            return Statistics::default();
        };

        let Ok(mut chain_stats) = stats.write() else {
            return Statistics::default();
        };

        chain_stats.reset_stats();

        let mut this_stats = chain_stats.clone();
        // Reset the current rollback stats.
        this_stats.live.rollbacks.live = rollbacks_reset(chain, RollbackType::LiveChain);
        this_stats.live.rollbacks.peer = rollbacks_reset(chain, RollbackType::Peer);
        this_stats.live.rollbacks.follower = rollbacks_reset(chain, RollbackType::Follower);

        this_stats
    }

    /// Return the statistics formatted as JSON
    #[must_use]
    pub fn as_json(&self, pretty: bool) -> String {
        let json = if pretty {
            serde_json::to_string_pretty(self)
        } else {
            serde_json::to_string(self)
        };
        match json {
            Ok(json) => json,
            Err(error) => {
                error!("{:?}", error);
                String::new()
            },
        }
    }
}

/// Count the invalidly deserialized blocks
pub(crate) fn stats_invalid_block(chain: Network, immutable: bool) {
    // This will actually always succeed.
    let Some(stats) = lookup_stats(chain) else {
        return;
    };

    let Ok(mut chain_stats) = stats.write() else {
        // Worst case if this fails (it never should) is we stop updating stats.
        error!("Stats RwLock should never be able to error.");
        return;
    };

    if immutable {
        chain_stats.mithril.invalid_blocks += 1;
    } else {
        chain_stats.live.invalid_blocks += 1;
    }
}

/// Count the validly deserialized blocks
pub(crate) fn new_live_block(
    chain: Network, total_live_blocks: u64, head_slot: u64, tip_slot: u64,
) {
    // This will actually always succeed.
    let Some(stats) = lookup_stats(chain) else {
        return;
    };

    let Ok(mut chain_stats) = stats.write() else {
        // Worst case if this fails (it never should) is we stop updating stats.
        error!("Stats RwLock should never be able to error.");
        return;
    };

    chain_stats.live.new_blocks += 1;
    chain_stats.live.blocks = total_live_blocks;
    chain_stats.live.head_slot = head_slot;
    chain_stats.live.tip = tip_slot;
}

/// Track the end of the current mithril update
pub(crate) fn new_mithril_update(
    chain: Network, mithril_tip: u64, total_live_blocks: u64, tip_slot: u64,
) {
    // This will actually always succeed.
    let Some(stats) = lookup_stats(chain) else {
        return;
    };

    let Ok(mut chain_stats) = stats.write() else {
        // Worst case if this fails (it never should) is we stop updating stats.
        error!("Stats RwLock should never be able to error.");
        return;
    };

    chain_stats.mithril.updates += 1;
    chain_stats.mithril.tip = mithril_tip;
    chain_stats.live.blocks = total_live_blocks;
    chain_stats.live.tip = tip_slot;
}

/// When did we start the backfill.
pub(crate) fn backfill_started(chain: Network) {
    // This will actually always succeed.
    let Some(stats) = lookup_stats(chain) else {
        return;
    };

    let Ok(mut chain_stats) = stats.write() else {
        // Worst case if this fails (it never should) is we stop updating stats.
        error!("Stats RwLock should never be able to error.");
        return;
    };

    // If we start another backfill, then that means the previous backfill failed, so record
    // it.
    if chain_stats.live.backfill_start.is_some() {
        chain_stats.live.backfill_failures += 1;
        chain_stats.live.backfill_failure_time = chain_stats.live.backfill_start;
    }

    chain_stats.live.backfill_start = Some(Utc::now());
}

/// When did we start the backfill.
pub(crate) fn backfill_ended(chain: Network, backfill_size: u64) {
    // This will actually always succeed.
    let Some(stats) = lookup_stats(chain) else {
        return;
    };

    let Ok(mut chain_stats) = stats.write() else {
        // Worst case if this fails (it never should) is we stop updating stats.
        error!("Stats RwLock should never be able to error.");
        return;
    };

    chain_stats.live.backfill_size = backfill_size;
    chain_stats.live.backfill_end = Some(Utc::now());
}

/// Track statistics about connections to the cardano peer node.
pub(crate) fn peer_connected(chain: Network, active: bool, peer_address: &str) {
    // This will actually always succeed.
    let Some(stats) = lookup_stats(chain) else {
        return;
    };

    let Ok(mut chain_stats) = stats.write() else {
        // Worst case if this fails (it never should) is we stop updating stats.
        error!("Stats RwLock should never be able to error.");
        return;
    };

    if active {
        chain_stats.live.reconnects += 1;
        chain_stats.live.last_connect = Utc::now();
        chain_stats.live.last_connected_peer = peer_address.to_string();
    } else {
        chain_stats.live.last_disconnect = Utc::now();
        chain_stats.live.last_disconnected_peer = peer_address.to_string();
    }

    chain_stats.live.connected = active;
}

/// Record when we started syncing
pub(crate) fn sync_started(chain: Network) {
    // This will actually always succeed.
    let Some(stats) = lookup_stats(chain) else {
        return;
    };

    let Ok(mut chain_stats) = stats.write() else {
        // Worst case if this fails (it never should) is we stop updating stats.
        error!("Stats RwLock should never be able to error.");
        return;
    };

    chain_stats.live.sync_start = Utc::now();
}

/// Record when we first reached tip. This can safely be called multiple times.
/// Except for overhead, only the first call will actually record the time.
pub(crate) fn tip_reached(chain: Network) {
    // This will actually always succeed.
    let Some(stats) = lookup_stats(chain) else {
        return;
    };

    let Ok(mut chain_stats) = stats.write() else {
        // Worst case if this fails (it never should) is we stop updating stats.
        error!("Stats RwLock should never be able to error.");
        return;
    };

    if chain_stats.live.sync_end.is_none() {
        chain_stats.live.sync_end = Some(Utc::now());
    }
}

/// Record that a Mithril snapshot Download has started.
pub(crate) fn mithril_dl_started(chain: Network) {
    // This will actually always succeed.
    let Some(stats) = lookup_stats(chain) else {
        return;
    };

    let Ok(mut chain_stats) = stats.write() else {
        // Worst case if this fails (it never should) is we stop updating stats.
        error!("Stats RwLock should never be able to error.");
        return;
    };

    chain_stats.mithril.dl_start = Utc::now();
}

/// Record when DL finished, if it fails, set size to None, otherwise the size of the
/// downloaded file.
pub(crate) fn mithril_dl_finished(chain: Network, dl_size: Option<u64>) {
    // This will actually always succeed.
    let Some(stats) = lookup_stats(chain) else {
        return;
    };

    let Ok(mut chain_stats) = stats.write() else {
        // Worst case if this fails (it never should) is we stop updating stats.
        error!("Stats RwLock should never be able to error.");
        return;
    };

    #[allow(clippy::cast_sign_loss)] // Its OK to cast the i64 to u64 because we clamped it.
    if let Some(dl_size) = dl_size {
        chain_stats.mithril.dl_end = Utc::now();
        chain_stats.mithril.dl_size = dl_size;
        let last_dl_duration = chain_stats.mithril.dl_end - chain_stats.mithril.dl_start;
        chain_stats.mithril.last_dl_duration =
            last_dl_duration.num_seconds().clamp(0, i64::MAX) as u64;
    } else {
        chain_stats.mithril.dl_failures += 1;
    }
}

/// Record that extracting the mithril snapshot archive has started.
pub(crate) fn mithril_extract_started(chain: Network) {
    // This will actually always succeed.
    let Some(stats) = lookup_stats(chain) else {
        return;
    };

    let Ok(mut chain_stats) = stats.write() else {
        // Worst case if this fails (it never should) is we stop updating stats.
        error!("Stats RwLock should never be able to error.");
        return;
    };

    chain_stats.mithril.extract_start = Utc::now();
}

/// Record when DL finished, if it fails, set size to None, otherwise the size of the
/// downloaded file.
pub(crate) fn mithril_extract_finished(
    chain: Network, extract_size: Option<u64>, deduplicated_size: u64, deduplicated_files: u64,
    changed_files: u64, new_files: u64,
) {
    // This will actually always succeed.
    let Some(stats) = lookup_stats(chain) else {
        return;
    };

    let Ok(mut chain_stats) = stats.write() else {
        // Worst case if this fails (it never should) is we stop updating stats.
        error!("Stats RwLock should never be able to error.");
        return;
    };

    if let Some(extract_size) = extract_size {
        chain_stats.mithril.extract_end = Utc::now();
        chain_stats.mithril.extract_size = extract_size;
        chain_stats.mithril.deduplicated_size = deduplicated_size;
        chain_stats.mithril.deduplicated = deduplicated_files;
        chain_stats.mithril.changed = changed_files;
        chain_stats.mithril.new = new_files;
    } else {
        chain_stats.mithril.extract_failures += 1;
    }
}

/// State of the Mithril cert validation.
#[derive(Copy, Clone)]
pub(crate) enum MithrilValidationState {
    /// Validation Started
    Start,
    /// Validation Failed
    Failed,
    /// Validation Finished
    Finish,
}

/// Record when Mithril Cert validation starts, ends or fails).
pub(crate) fn mithril_validation_state(chain: Network, mithril_state: MithrilValidationState) {
    // This will actually always succeed.
    let Some(stats) = lookup_stats(chain) else {
        return;
    };

    let Ok(mut chain_stats) = stats.write() else {
        // Worst case if this fails (it never should) is we stop updating stats.
        error!("Stats RwLock should never be able to error.");
        return;
    };

    match mithril_state {
        MithrilValidationState::Start => chain_stats.mithril.validate_start = Utc::now(),
        MithrilValidationState::Failed => chain_stats.mithril.validate_failures += 1,
        MithrilValidationState::Finish => chain_stats.mithril.validate_end = Utc::now(),
    }
}

// -------- ROLLBACK STATISTIC TRACKING
// ----------------------------------------------------------

/// The types of rollbacks we track for a chain.
#[derive(EnumIter, Eq, Ord, PartialEq, PartialOrd, Copy, Clone)]
pub enum RollbackType {
    /// Rollback on the in-memory live chain.
    LiveChain,
    /// Rollback signaled by the peer.
    Peer,
    /// Rollback synthesized for the Follower.
    Follower,
}

/// Individual rollback records.
type RollbackRecords = SkipMap<u64, Rollback>;
/// Rollback Records per rollback type.
type RollbackTypeMap = SkipMap<RollbackType, Arc<RwLock<RollbackRecords>>>;
/// Record of rollbacks.
type RollbackMap = SkipMap<Network, RollbackTypeMap>;
/// Statistics of rollbacks detected per chain.
static ROLLBACKS_MAP: Lazy<RollbackMap> = Lazy::new(|| {
    let map = RollbackMap::new();
    for network in Network::iter() {
        let type_map = RollbackTypeMap::new();
        for rollback in RollbackType::iter() {
            type_map.insert(rollback, Arc::new(RwLock::new(RollbackRecords::new())));
        }
        map.insert(network, type_map);
    }
    map
});

/// Get the actual rollback map for a chain.
fn lookup_rollback_map(
    chain: Network, rollback: RollbackType,
) -> Option<Arc<RwLock<RollbackRecords>>> {
    let Some(chain_rollback_map) = ROLLBACKS_MAP.get(&chain) else {
        error!("Rollback stats SHOULD BE exhaustively pre-allocated.");
        return None;
    };
    let chain_rollback_map = chain_rollback_map.value();

    let Some(rollback_map) = chain_rollback_map.get(&rollback) else {
        error!("Rollback stats SHOULD BE exhaustively pre-allocated.");
        return None;
    };
    let rollback_map = rollback_map.value();

    Some(rollback_map.clone())
}

/// Extract the current rollback stats as a vec.
fn rollbacks(chain: Network, rollback: RollbackType) -> Vec<Rollback> {
    let Some(rollback_map) = lookup_rollback_map(chain, rollback) else {
        return Vec::new();
    };

    let Ok(rollback_values) = rollback_map.read() else {
        error!("Rollback stats LOCK Poisoned, should not happen.");
        return vec![];
    };

    let mut rollbacks = Vec::new();

    // Get all the rollback stats.
    for stat in rollback_values.iter() {
        rollbacks.push(stat.value().clone());
    }

    rollbacks
}

/// Reset ALL the rollback stats for a given blockchain.
fn rollbacks_reset(chain: Network, rollback: RollbackType) -> Vec<Rollback> {
    let Some(rollback_map) = lookup_rollback_map(chain, rollback) else {
        return Vec::new();
    };

    let Ok(rollbacks) = rollback_map.write() else {
        error!("Rollback stats LOCK Poisoned, should not happen.");
        return vec![];
    };

    rollbacks.clear();

    Vec::new()
}

/// Count a rollback
pub(crate) fn rollback(chain: Network, rollback: RollbackType, depth: u64) {
    let Some(rollback_map) = lookup_rollback_map(chain, rollback) else {
        return;
    };

    let Ok(rollbacks) = rollback_map.write() else {
        error!("Rollback stats LOCK Poisoned, should not happen.");
        return;
    };

    let mut value = match rollbacks.get(&depth) {
        Some(value_entry) => (*value_entry.value()).clone(),
        None => Rollback { depth, count: 0 },
    };

    value.count += 1;

    let _unused = rollbacks.insert(depth, value);
}
