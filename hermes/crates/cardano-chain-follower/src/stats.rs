//! Cardano Chain Follower Statistics

use std::sync::RwLock;

use chrono::{DateTime, Utc};
use crossbeam_skiplist::SkipMap;
use once_cell::sync::Lazy;
use strum::IntoEnumIterator;

use crate::Network;

/// Statistics related to Mithril Snapshots
#[derive(Debug, Default, Clone)]
pub struct Mithril {
    /// Number of Mithril Snapshots that have downloaded successfully.
    pub updates: u64,
    /// Number of blocks in current snapshot
    pub blocks: u64,
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
#[derive(Debug, Default, Clone)]
pub struct Rollback {
    /// How deep was the rollback from tip.
    pub depth: u64,
    /// How many times has a rollback been this deep.
    pub count: u64,
}

/// Statistics related to the live blockchain
#[derive(Debug, Default, Clone)]
pub struct Live {
    /// The Time that synchronization to this blockchain started
    pub sync_start: DateTime<Utc>,
    /// The Time that synchronization to this blockchain was complete up-to-tip. None =
    /// Not yet synchronized.
    pub sync_end: Option<DateTime<Utc>>,
    /// Backfill size to achieve synchronization. (0 before sync completed)
    pub backfill_size: u64,
    /// Current Number of Live Blocks
    pub blocks: u64,
    /// The current live tip slot#
    pub tip: u64,
    /// Number of times we connected/re-connected to the Node.
    pub reconnects: u64,
    /// Last reconnect time,
    pub last_connect: DateTime<Utc>,
    /// Is there an active connection to the node
    pub connected: bool,
    /// Rollback statistics - Vec is sorted by depth, ascending.
    pub rollbacks: Vec<Rollback>,
    /// New blocks read from blockchain.
    pub new_blocks: u64,
    /// Blocks that failed to deserialize from the blockchain.
    pub invalid_blocks: u64,
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
#[derive(Debug, Default, Clone)]
pub struct Statistics {
    /// Statistics related to the live connection to the blockchain.
    pub live: Live,
    /// Statistics related to the mithril certified blockchain archive.
    pub mithril: Mithril,
}

/// Type we use to manage the Sync Task handle map.
type StatsMap = SkipMap<Network, RwLock<Statistics>>;
/// The statistics being maintained per chain.
static STATS_MAP: Lazy<StatsMap> = Lazy::new(|| {
    let map = SkipMap::new();
    for network in Network::iter() {
        map.insert(network, RwLock::new(Statistics::default()));
    }
    map
});

/// Record of rollbacks.
type RollbackMap = SkipMap<Network, SkipMap<u64, Rollback>>;
/// Statistics of rollbacks detected per chain.
static ROLLBACKS_MAP: Lazy<RollbackMap> = Lazy::new(|| {
    let map = SkipMap::new();
    for network in Network::iter() {
        map.insert(network, SkipMap::new());
    }
    map
});

/// Extract the current rollback stats as a vec.
fn rollbacks(chain: Network) -> Vec<Rollback> {
    #[allow(clippy::expect_used)] // Exhaustively pre-allocated.
    let chain_entry = ROLLBACKS_MAP
        .get(&chain)
        .expect("Rollback stats are exhaustively pre-allocated.");
    let rollback_map = chain_entry.value();

    let mut rollbacks = Vec::new();

    // Get all the rollback stats.
    for stat in rollback_map {
        rollbacks.push((*stat.value()).clone());
    }

    rollbacks
}

/// Reset the rollback stats for a given blockchain.
fn rollbacks_reset(chain: Network) -> Vec<Rollback> {
    let _chain_entry = ROLLBACKS_MAP.insert(chain, SkipMap::new());

    Vec::new()
}

/// Count a rollback
/// As we ONLY call this in a single place, from a single thread, it is free of data
/// races.
#[allow(dead_code)]
pub(crate) fn rollback(chain: Network, depth: u64) {
    #[allow(clippy::expect_used)] // Exhaustively pre-allocated.
    let chain_entry = ROLLBACKS_MAP
        .get(&chain)
        .expect("Rollback stats are exhaustively pre-allocated.");

    let chain_map = chain_entry.value();

    let mut value = match chain_map.get(&depth) {
        Some(value_entry) => (*value_entry.value()).clone(),
        None => Rollback { depth, count: 0 },
    };

    value.count += 1;

    let _unused = chain_map.insert(depth, value);
}

impl Statistics {
    /// Get a new statistics struct for a given blockchain network.
    #[allow(clippy::missing_panics_doc)] // Can't actually panic.
    pub fn new(chain: Network) -> Self {
        #[allow(clippy::expect_used)] // Exhaustively pre-allocated.
        let chain_entry = STATS_MAP
            .get(&chain)
            .expect("Stats are exhaustively pre-allocated.");
        #[allow(clippy::expect_used)] // Mutex not used iteratively.
        let chain_stats = chain_entry
            .value()
            .read()
            .expect("Stats not read recursively.");
        let mut this_stats = chain_stats.clone();
        // Set the current rollback stats.
        this_stats.live.rollbacks = rollbacks(chain);
        this_stats
    }

    /// Reset the incremental counters in a stats record.
    fn reset_stats(&mut self) {
        self.live.reset();
        self.mithril.reset();
    }

    /// Reset amd return cumulative counters contained in the statistics.
    #[allow(clippy::missing_panics_doc)] // Can't actually panic.
    pub fn reset(chain: Network) -> Self {
        #[allow(clippy::expect_used)] // Exhaustively pre-allocated.
        let chain_entry = STATS_MAP
            .get(&chain)
            .expect("Stats are exhaustively pre-allocated.");
        #[allow(clippy::expect_used)] // Mutex not used iteratively.
        let mut chain_stats = chain_entry
            .value()
            .write()
            .expect("Stats not written recursively.");
        chain_stats.reset_stats();
        let mut this_stats = chain_stats.clone();
        // Reset the current rollback stats.
        this_stats.live.rollbacks = rollbacks_reset(chain);
        this_stats
    }
}

/// Count the invalidly deserialized blocks
#[allow(clippy::missing_panics_doc)] // Can't actually panic.
pub(crate) fn stats_invalid_block(chain: Network, immutable: bool) {
    #[allow(clippy::expect_used)] // Exhaustively pre-allocated.
    let chain_entry = STATS_MAP
        .get(&chain)
        .expect("Stats are exhaustively pre-allocated.");
    #[allow(clippy::expect_used)] // Mutex not used iteratively.
    let mut chain_stats = chain_entry
        .value()
        .write()
        .expect("Stats not written recursively.");

    if immutable {
        chain_stats.mithril.invalid_blocks += 1;
    } else {
        chain_stats.live.invalid_blocks += 1;
    }
}
