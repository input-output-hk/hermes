//! Data about the current Mithril Snapshot
use std::{default, sync::LazyLock};

use dashmap::DashMap;

use crate::{network::Network, snapshot_id::SnapshotId};

/// Current Mithril Snapshot Data for a network.
#[derive(Debug, Clone)]
pub(crate) struct SnapshotData {
    /// Snapshot ID the data represents
    id: SnapshotId,
}

impl SnapshotData {
    /// Create a new Snapshot Data.
    pub(crate) fn new(id: SnapshotId) -> Self {
        SnapshotData { id }
    }

    /// Get the snapshot ID of this Snapshot Data.
    pub(crate) fn id(&self) -> &SnapshotId {
        &self.id
    }
}

impl default::Default for SnapshotData {
    /// The default snapshot data represents there is no latest snapshot.
    fn default() -> Self {
        SnapshotData {
            id: SnapshotId::default(),
        }
    }
}

/// Current Mithril Snapshot for a network.
static CURRENT_MITHRIL_SNAPSHOT: LazyLock<DashMap<Network, SnapshotData>> =
    LazyLock::new(DashMap::new);

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
pub(crate) fn update_latest_mithril_snapshot(chain: Network, snapshot_id: SnapshotId) {
    let snapshot_data = SnapshotData::new(snapshot_id);

    // Save the current snapshot
    CURRENT_MITHRIL_SNAPSHOT.insert(chain, snapshot_data);
}
