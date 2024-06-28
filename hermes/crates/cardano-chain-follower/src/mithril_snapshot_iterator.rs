//! Internal Mithril snapshot iterator functions.

use std::path::Path;

use pallas::network::miniprotocols::Point;
use pallas_hardano::storage::immutable::FallibleBlock;
use tracing::error;

use crate::{
    error::{Error, Result},
    network::Network,
    MultiEraBlock,
};

/// Search backwards by 120 slots (seconds) looking for a previous block.
/// The previous block should be approximately 20 slots earlier, this gives us a
/// reasonable window to search of ~6 blocks.
const BACKWARD_SEARCH_SLOT_INTERVAL: u64 = 120;

/// Wraps the iterator type returned by Pallas.
pub(crate) struct MithrilSnapshotIterator {
    /// The chain being iterated
    chain: Network,
    /// Where we really want to start iterating from
    start: Point,
    /// Previous iteration point.
    previous: Option<Point>,
    /// Inner iterator.
    inner: Box<dyn Iterator<Item = FallibleBlock> + Send + Sync>,
}

/// Create a probe point used in iterations to find the start when its not exactly known.
pub(crate) fn probe_point(point: &Point, distance: u64) -> Point {
    // Now that we have the tip, step back about 4 block intervals from tip, and do a fuzzy
    // iteration to find the exact two blocks at the end of the immutable chain.
    let step_back_search = point.slot_or_default().saturating_sub(distance);

    // We stepped back to the origin, so just return Origin
    if step_back_search == 0 {
        return Point::Origin;
    }

    // Create a fuzzy search probe by making the hash zero length.
    Point::Specific(step_back_search, Vec::new())
}

impl MithrilSnapshotIterator {
    /// Create a mithril iterator, optionally where we know the previous point.
    ///
    /// # Arguments
    ///
    /// `chain`: The blockchain network to iterate.
    /// `from`: The point to start iterating from.  If the `Point` does not contain a
    /// hash, the iteration start is fuzzy. `previous`: The previous point we are
    /// iterating, if known.    If the previous is NOT known, then the first block
    /// yielded by the iterator is discarded and becomes the known previous.
    pub(crate) fn new(
        chain: Network, path: &Path, from: &Point, previous_point: Option<Point>,
    ) -> Result<Self> {
        let actual_start = match previous_point {
            Some(_) => from.clone(),
            None => probe_point(from, BACKWARD_SEARCH_SLOT_INTERVAL),
        };

        let iterator =
            pallas_hardano::storage::immutable::read_blocks_from_point(path, actual_start)
                .map_err(|error| Error::MithrilSnapshot(Some(error)))?;
        Ok(MithrilSnapshotIterator {
            chain,
            start: from.clone(),
            previous: previous_point,
            inner: iterator,
        })
    }
}

impl Iterator for MithrilSnapshotIterator {
    type Item = MultiEraBlock;

    fn next(&mut self) -> Option<Self::Item> {
        for maybe_block in self.inner.by_ref() {
            if let Ok(block) = maybe_block {
                if let Some(previous) = self.previous.clone() {
                    // We can safely fully decode this block.
                    if let Ok(block_data) = MultiEraBlock::new(self.chain, block, &previous, true) {
                        // Update the previous point
                        self.previous = Some(block_data.point());

                        // Make sure we got to the start, otherwise this could be a block artifact
                        // from a discover previous point search.
                        if block_data < self.start {
                            continue;
                        }

                        return Some(block_data);
                    }
                    error!("Error decoding a block from the snapshot");
                    break;
                }

                // We cannot fully decode this block because we don't know its previous point,
                // So this MUST be the first block in iteration, so use it as the previous.
                if let Ok(raw_decoded_block) =
                    pallas::ledger::traverse::MultiEraBlock::decode(&block)
                {
                    self.previous = Some(Point::Specific(
                        raw_decoded_block.slot(),
                        raw_decoded_block.hash().to_vec(),
                    ));
                    continue;
                }
                error!("Error decoding block to use for previous from the snapshot.");
                break;
            }

            error!("Error while fetching a block from the snapshot");
            break;
        }

        None
    }
}
