//! Internal Mithril snapshot iterator functions.

use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::task;
use tracing::{debug, error};

use crate::{
    error::Result,
    mithril_query::{make_mithril_iterator, ImmutableBlockIterator},
    network::Network,
    point::ORIGIN_POINT,
    MultiEraBlock, Point,
};

/// Search backwards by 60 slots (seconds) looking for a previous block.
/// This search window is doubled until the search succeeds.
const BACKWARD_SEARCH_SLOT_INTERVAL: u64 = 60;

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
    /// Try and probe to establish the iterator from the desired point.
    async fn try_fuzzy_iterator(
        chain: Network, path: &Path, from: &Point, search_interval: u64,
    ) -> Option<MithrilSnapshotIterator> {
        let point = probe_point(from, search_interval);
        let Ok(mut iterator) = make_mithril_iterator(path, &point).await else {
            return None;
        };

        let mut previous = None;
        let mut this = None;

        loop {
            let next = iterator.next();

            match next {
                Some(Ok(raw_block)) => {
                    let Ok(block) = pallas::ledger::traverse::MultiEraBlock::decode(&raw_block)
                    else {
                        return None;
                    };

                    let point = Point::new(block.slot(), block.hash().to_vec());
                    previous = this;
                    this = Some(point.clone());

                    debug!("Searching for {from}. {this:?} > {previous:?}");

                    // Stop as soon as we find the point, or exceed it.
                    if point >= *from {
                        break;
                    }
                },
                Some(Err(err)) => {
                    error!("Error while iterating fuzzy mithril data: {}", err);
                    return None;
                },
                None => break,
            };
        }

        debug!("Best Found for {from}. {this:?} > {previous:?}");

        // Fail if we didn't find the destination block, or its immediate predecessor.
        previous.as_ref()?;
        let this = this?;

        // Remake the iterator, based on the new known point.
        let Ok(iterator) = make_mithril_iterator(path, &this).await else {
            return None;
        };

        Some(MithrilSnapshotIterator {
            inner: Arc::new(Mutex::new(MithrilSnapshotIteratorInner {
                chain,
                start: this,
                previous,
                inner: iterator,
            })),
        })
    }

    /// Do a fuzzy search to establish the iterator.
    /// We use this when we don;t know the previous point, and need to find it.
    async fn fuzzy_iterator(chain: Network, path: &Path, from: &Point) -> MithrilSnapshotIterator {
        let mut backwards_search = BACKWARD_SEARCH_SLOT_INTERVAL;
        loop {
            if let Some(iterator) =
                Self::try_fuzzy_iterator(chain, path, from, backwards_search).await
            {
                return iterator;
            }

            backwards_search *= 2;
        }
    }

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
        if previous_point.is_none() {
            return Ok(Self::fuzzy_iterator(chain, path, from).await);
        }

        debug!("Actual Mithril Iterator Start: {}", from);

        let iterator = make_mithril_iterator(path, from).await?;

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
            let next_block = inner_iterator.next();
            debug!("next block: {:?}", next_block);
            next_block
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
                    if let Ok(block_data) = MultiEraBlock::new(self.chain, block, &previous, 0) {
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
