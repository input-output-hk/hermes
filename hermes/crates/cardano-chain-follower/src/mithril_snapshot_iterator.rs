//! Internal Mithril snapshot iterator functions.

use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::task;
use tracing::error;

use crate::{
    error::Result,
    mithril_query::{make_mithril_iterator, ImmutableBlockIterator},
    network::Network,
    point::ORIGIN_POINT,
    MultiEraBlock, Point,
};

/// Search backwards by 120 slots (seconds) looking for a previous block.
/// The previous block should be approximately 20 slots earlier, this gives us a
/// reasonable window to search of ~6 blocks.
const BACKWARD_SEARCH_SLOT_INTERVAL: u64 = 120;

/// Synchronous Inner Iterator state
struct MithrilSnapshotIteratorInner {
    /// The chain being iterated
    chain: Network,
    /// Where we really want to start iterating from
    start: Point,
    /// Previous iteration point.
    previous: Option<Point>,
    /// Inner iterator.
    inner: ImmutableBlockIterator,
}

/// Wraps the iterator type returned by Pallas.
pub(crate) struct MithrilSnapshotIterator {
    /// Inner Mutable Synchronous Iterator State
    inner: Arc<Mutex<MithrilSnapshotIteratorInner>>,
}

/// Create a probe point used in iterations to find the start when its not exactly known.
pub(crate) fn probe_point(point: &Point, distance: u64) -> Point {
    // Now that we have the tip, step back about 4 block intervals from tip, and do a fuzzy
    // iteration to find the exact two blocks at the end of the immutable chain.
    let step_back_search = point.slot_or_default().saturating_sub(distance);

    // We stepped back to the origin, so just return Origin
    if step_back_search == 0 {
        return ORIGIN_POINT;
    }

    // Create a fuzzy search probe by making the hash zero length.
    Point::fuzzy(step_back_search)
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
    pub(crate) async fn new(
        chain: Network, path: &Path, from: &Point, previous_point: Option<Point>,
    ) -> Result<Self> {
        let actual_start = match previous_point {
            Some(_) => from.clone(),
            None => probe_point(from, BACKWARD_SEARCH_SLOT_INTERVAL),
        };

        let iterator = make_mithril_iterator(path, &actual_start).await?;

        Ok(MithrilSnapshotIterator {
            inner: Arc::new(Mutex::new(MithrilSnapshotIteratorInner {
                chain,
                start: from.clone(),
                previous: previous_point,
                inner: iterator,
            })),
        })
    }

    /// Get the next block, in a way that is Async friendly.
    /// Returns the next block, or None if there are no more blocks.
    pub(crate) async fn next(&self) -> Option<MultiEraBlock> {
        let inner = self.inner.clone();

        let res = task::spawn_blocking(move || {
            #[allow(clippy::unwrap_used)] // Unwrap is safe here because the lock can't be poisoned.
            let mut inner_iterator = inner.lock().unwrap();
            inner_iterator.next()
        })
        .await;

        match res {
            Ok(res) => res,
            Err(_error) => None,
        }
    }
}

impl Iterator for MithrilSnapshotIteratorInner {
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
                    self.previous = Some(Point::new(
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
