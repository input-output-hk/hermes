//! Data about the current Mithril Snapshot
use std::{
    default,
    path::{Path, PathBuf},
    sync::Arc,
};

use dashmap::DashMap;
use once_cell::sync::Lazy;
use pallas_hardano::storage::immutable::Point;

use crate::{network::Network, snapshot_id::SnapshotId};

/// Raw Blake3 hash of a file. (Don't need constant time comparison)
pub(crate) type RawHash = [u8; 32];
/// Map of all files to their respective hashes. 9Used for dedup).
pub(crate) type FileHashMap = DashMap<PathBuf, RawHash>;

/// Current Mithril Snapshot Data for a network.
#[derive(Debug, Clone)]
pub(crate) struct SnapshotData {
    /// Snapshot ID the data represents
    id: SnapshotId,
    /// Hashmap of all files in the snapshot.
    hash_map: Arc<FileHashMap>,
}

impl SnapshotData {
    /// Create a new Snapshot Data.
    pub(crate) fn new(id: SnapshotId, hash_map: Arc<FileHashMap>) -> Self {
        SnapshotData { id, hash_map }
    }

    /// Does this snapshot ID actually exist.
    pub(crate) fn exists(&self) -> bool {
        self.id.tip() != Point::Origin
    }

    /// Get the snapshot ID of this Snapshot Data.
    pub(crate) fn id(&self) -> &SnapshotId {
        &self.id
    }

    /// Get a current hash for a known file in this snapshot data (or None if its not
    /// known).
    pub(crate) fn current_hash(&self, filename: &Path) -> Option<RawHash> {
        let entry = self.hash_map.get(filename)?;
        Some(*entry.value())
    }
}

impl default::Default for SnapshotData {
    /// The default snapshot data represents there is no latest snapshot.
    fn default() -> Self {
        SnapshotData {
            id: SnapshotId::default(),
            hash_map: FileHashMap::default().into(),
        }
    }
}

/// Current Mithril Snapshot for a network.
static CURRENT_MITHRIL_SNAPSHOT: Lazy<DashMap<Network, SnapshotData>> = Lazy::new(DashMap::new);

/// Get the current latest snapshot data we have recorded.
pub(crate) fn latest_mithril_snapshot_data(chain: Network) -> SnapshotData {
    // There should ALWAYS be a snapshot for the chain if this is called.

    match CURRENT_MITHRIL_SNAPSHOT.get(&chain) {
        Some(snapshot_data) => snapshot_data.value().clone(),
        None => SnapshotData::default(),
    }
}

/// Get the latest Mithril Snapshot for a network.
pub(crate) fn latest_mithril_snapshot_id(chain: Network) -> SnapshotId {
    // There should ALWAYS be a snapshot for the chain if this is called.
    latest_mithril_snapshot_data(chain).id
}

/// Update the latest snapshot data.
pub(crate) fn update_latest_mithril_snapshot(
    chain: Network, snapshot_id: SnapshotId, hash_map: Arc<FileHashMap>,
) {
    let snapshot_data = SnapshotData::new(snapshot_id, hash_map);

    // Save the current snapshot
    CURRENT_MITHRIL_SNAPSHOT.insert(chain, snapshot_data);
}
