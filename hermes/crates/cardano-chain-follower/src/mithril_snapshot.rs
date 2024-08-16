//! Internal Mithril snapshot functions.

use logcall::logcall;
use tracing_log::log;

use crate::{
    mithril_snapshot_data::latest_mithril_snapshot_id,
    mithril_snapshot_iterator::MithrilSnapshotIterator, network::Network, MultiEraBlock, Point,
};

// Any single program using this crate can have EXACTLY THREE Mithril snapshots.
// One, for each of the known networks.
// If more mithril snapshots are configured, then the crate will error.
// It IS possible to redundantly configure mithril snapshots, provided they are
// identically configured. The only config option that can change, is if the snapshot is
// auto-updated, ANY follower which sets this enables this function and it can not be
// disabled once started without stopping the program.

/// Holds information about a Mithril snapshot.
#[derive(Clone)]
pub(crate) struct MithrilSnapshot {
    /// Network that this snapshot is configured for
    chain: Network,
}

impl MithrilSnapshot {
    /// Create a new Mithril Snapshot handler
    pub(crate) fn new(chain: Network) -> Self {
        Self { chain }
    }

    /// Checks if the snapshot contains a given point.
    ///
    /// # Arguments
    /// * `network`: The network that this function should check against.
    /// * `point`: The point to be checked for existence within the specified Mithril
    ///   snapshot.
    ///
    /// Returns true if the point exists within the Mithril snapshot for the specified
    /// network, false otherwise.
    pub(crate) fn contains_point(&self, point: &Point) -> bool {
        let latest_id = latest_mithril_snapshot_id(self.chain);

        point.slot_or_default() <= latest_id.tip().slot_or_default()
    }

    /// Tries get an iterator that reads blocks from the Mithril snapshot from a given
    /// point. Returns None if the point is not contained in the snapshot.
    ///
    /// # Arguments
    ///
    /// * `point`: Start point.
    ///
    /// # Errors
    ///
    /// Returns None if its not possible to iterate a mithril snapshot from the requested
    /// point for ANY reason.
    #[allow(clippy::indexing_slicing)]
    #[logcall("debug")]
    pub(crate) async fn try_read_blocks_from_point(
        &self, point: &Point,
    ) -> Option<MithrilSnapshotIterator> {
        let snapshot_id = latest_mithril_snapshot_id(self.chain);
        let snapshot_path = snapshot_id.immutable_path();

        // Quick check if the block can be within the immutable data.
        if !self.contains_point(point) {
            return None;
        }

        // We don't know the previous block, so we need to find it.
        MithrilSnapshotIterator::new(self.chain, &snapshot_path, point, None)
            .await
            .ok()
    }

    /// Read a single block from a known point.
    #[allow(clippy::indexing_slicing)]
    #[logcall("debug")]
    pub(crate) async fn read_block_at(&self, point: &Point) -> Option<MultiEraBlock> {
        if let Some(iterator) = self.try_read_blocks_from_point(point).await {
            let block = iterator.next().await;
            return block;
        }
        None
    }
}

#[cfg(test)]
mod tests {}
