//! Internal Mithril snapshot functions.

use crate::{error::Result, multi_era_block_data::MultiEraBlockData, network::Network};

use dashmap::DashMap;
use once_cell::sync::Lazy;
use pallas::network::miniprotocols::Point;
use pallas_hardano::storage::immutable::FallibleBlock;
use tracing::debug;

/// Wraps the iterator type returned by Pallas.
pub(crate) struct MithrilSnapshotIterator {
    /// Inner iterator.
    inner: Box<dyn Iterator<Item = FallibleBlock> + Send + Sync>,
}

impl Iterator for MithrilSnapshotIterator {
    type Item = FallibleBlock;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

// Any single program using this crate can have EXACTLY THREE Mithril snapshots.
// One, for each of the known networks.
// If more mithril snapshots are configured, then the crate will error.
// It IS possible to redundantly configure mithril snapshots, provided they are
// identically configured. The only config option that can change, is if the snapshot is
// auto-updated, ANY follower which sets this enables this function and it can not be
// disabled once started without stopping the program.

/// Current TIP of a network.
static CURRENT_TIPS: Lazy<DashMap<Network, Point>> = Lazy::new(DashMap::new);

/// Try and update the current tip from an existing snapshot.
#[allow(dead_code)]
pub(crate) fn update_tip(chain: Network, tip: Point) {
    debug!(
        "Updating TIP from Immutable storage for {} to {:?}",
        chain, tip
    );

    CURRENT_TIPS.insert(chain, tip);
}

/// Holds information about a Mithril snapshot.
#[derive(Clone)]
pub(crate) struct MithrilSnapshot;

impl MithrilSnapshot {
    /// Get the current tip of the configured Mithril Network
    #[allow(dead_code)]
    pub fn tip(network: Network) -> Option<Point> {
        CURRENT_TIPS
            .get(&network)
            .map(|entry| entry.value().clone())
    }

    /// Tries reading a block from the Mithril snapshot. Returns None if the point
    /// is not contained in the snapshot.
    ///
    /// # Arguments
    ///
    /// * `point`: Point at which to read the block.
    ///
    /// # Errors
    ///
    /// Returns Err if anything fails while reading the block data.
    #[allow(clippy::unnecessary_wraps)]
    pub fn try_read_block(_network: Network, _point: &Point) -> Result<Option<MultiEraBlockData>> {
        /* TODO(SJ)  : Fix This
        if let Some(mithril_path) = read_mithril_immutable_path(network) {
            if !MithrilSnapshot::contains_point(network, &point) {
                return Ok(None);
            }

            let mut block_data_iter =
                pallas_hardano::storage::immutable::read_blocks_from_point(&mithril_path, point)
                    .map_err(|error| Error::MithrilSnapshot(Some(error)))?;

            match block_data_iter.next() {
                Some(res) => {
                    let block_data = res.map_err(Error::MithrilSnapshotChunk)?;

                    Ok(Some(MultiEraBlockData::new(block_data)))
                },
                None => Ok(None),
            }
        } else {  */
        Ok(None)
        /*  } */
    }

    /// Tries reading a range of blocks from the Mithril snapshot.
    /// Returns None if the range is not contained in the snapshot.
    ///
    /// This returns the last point that was read. This is useful to check
    /// if the range was partially read.
    ///
    /// # Arguments
    ///
    /// * `from`: Start point.
    /// * `to`: End point.
    ///
    /// # Errors
    ///
    /// Returns Err if anything fails while reading any block's data.
    #[allow(clippy::unnecessary_wraps)]
    pub fn try_read_block_range(
        _network: Network, _from: &Point, _to: &Point,
    ) -> Result<Option<(Point, Vec<MultiEraBlockData>)>> {
        /* TODO(SJ)  : Fix This
        if let Some(mithril_path) = read_mithril_immutable_path(network) {
            if !MithrilSnapshot::contains_point(network, &from) {
                return Ok(None);
            }

            let blocks_iter =
                pallas_hardano::storage::immutable::read_blocks_from_point(&mithril_path, from)
                    .map_err(|error| Error::MithrilSnapshot(Some(error)))?;

            let mut block_data_vec = Vec::new();
            for result in blocks_iter {
                let block_data =
                    MultiEraBlockData::new(result.map_err(Error::MithrilSnapshotChunk)?);

                // TODO(fsgr): Should we check the hash as well?
                //             Maybe throw an error if we don't get the block we were expecting at
                // that             slot?
                if block_data.decode()?.slot() > to.slot_or_default() {
                    break;
                }

                block_data_vec.push(block_data);
            }

            // Get the point from last block read.
            // Pop here to get an owned value (we'll insert it back later).
            match block_data_vec.pop() {
                Some(last_block_data) => {
                    let last_block = last_block_data.decode()?;
                    let last_block_point =
                        Point::new(last_block.slot(), last_block.hash().to_vec());

                    // Push the last block data back
                    block_data_vec.push(last_block_data);

                    Ok(Some((last_block_point, block_data_vec)))
                },
                None => Ok(None),
            }
        } else {
            Ok(None)
        }*/
        Ok(None)
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
    pub fn try_read_blocks_from_point(
        _network: Network, _point: &Point,
    ) -> Option<MithrilSnapshotIterator> {
        /* TODO(SJ)  : Fix This

        if let Some(mithril_path) = read_mithril_immutable_path(network) {
            if MithrilSnapshot::contains_point(network, &point) {
                let iter = pallas_hardano::storage::immutable::read_blocks_from_point(
                    &mithril_path,
                    point,
                )
                .map_err(|_| Error::MithrilSnapshot)
                .ok()?;

                Some(MithrilSnapshotIterator { inner: iter })
            } else {
                None
            }
        } else {
            None
        }*/
        None
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
    #[allow(dead_code)]
    pub fn contains_point(network: Network, point: &Point) -> bool {
        if let Some(tip) = MithrilSnapshot::tip(network) {
            point.slot_or_default() <= tip.slot_or_default()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {}
