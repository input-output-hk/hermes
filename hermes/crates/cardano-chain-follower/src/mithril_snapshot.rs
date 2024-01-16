//! Internal Mithril snapshot functions.

use std::path::PathBuf;

use pallas::network::miniprotocols::Point;
use pallas_hardano::storage::immutable::FallibleBlock;

use crate::{Error, MultiEraBlockData, Result};

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

/// Holds information about a Mithril snapshot.
pub(crate) struct MithrilSnapshot {
    /// Path to the Mithril snapshot.
    pub path: PathBuf,
    /// Snapshot's tip.
    pub tip: Point,
}

impl MithrilSnapshot {
    /// Gets information about the snapshot at the given path.
    ///
    /// # Arguments
    ///
    /// * `path`: Mithril snapshot path.
    ///
    /// # Errors
    ///
    /// Returns Err if it can't read where the tip is at in the snapshot or
    /// if reading the snapshot files fails.
    pub fn from_path(path: PathBuf) -> Result<Self> {
        let tip = pallas_hardano::storage::immutable::get_tip(&path)
            .map_err(|_| Error::MithrilSnapshot)?
            .ok_or(Error::MithrilSnapshot)?;

        Ok(Self { path, tip })
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
    pub fn try_read_block(&self, point: Point) -> Result<Option<MultiEraBlockData>> {
        if !self.contains_point(&point) {
            return Ok(None);
        }

        let mut block_data_iter =
            pallas_hardano::storage::immutable::read_blocks_from_point(&self.path, point)
                .map_err(|_| Error::MithrilSnapshot)?;

        match block_data_iter.next() {
            Some(res) => {
                let block_data = res.map_err(|_| Error::MithrilSnapshot)?;

                Ok(Some(MultiEraBlockData(block_data)))
            },
            None => Ok(None),
        }
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
    #[allow(clippy::needless_pass_by_value)]
    pub fn try_read_block_range(
        &self, from: Point, to: Point,
    ) -> Result<Option<(Point, Vec<MultiEraBlockData>)>> {
        if !self.contains_point(&from) {
            return Ok(None);
        }

        let blocks_iter =
            pallas_hardano::storage::immutable::read_blocks_from_point(&self.path, from)
                .map_err(|_| Error::MithrilSnapshot)?;

        let mut block_data_vec = Vec::new();
        for result in blocks_iter {
            let block_data = MultiEraBlockData(result.map_err(|_| Error::MithrilSnapshot)?);

            // TODO(fsgr): Should we check the hash as well?
            //             Maybe throw an error if we don't get the block we were expecting at that
            //             slot?
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
                let last_block_point = Point::new(last_block.slot(), last_block.hash().to_vec());

                // Push the last block data back
                block_data_vec.push(last_block_data);

                Ok(Some((last_block_point, block_data_vec)))
            },
            None => Ok(None),
        }
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
    /// Returns Err if anything fails while trying to find the starting point in the
    /// snapshot.
    pub fn try_read_blocks_from_point(&self, point: Point) -> Option<MithrilSnapshotIterator> {
        if !self.contains_point(&point) {
            return None;
        }

        let iter = pallas_hardano::storage::immutable::read_blocks_from_point(&self.path, point)
            .map_err(|_| Error::MithrilSnapshot)
            .ok()?;

        Some(MithrilSnapshotIterator { inner: iter })
    }

    /// Naively checks if the snapshot contains a point.
    ///
    /// # Arguments
    ///
    /// * `point`: Point to check.
    pub fn contains_point(&self, point: &Point) -> bool {
        point.slot_or_default() <= self.tip.slot_or_default()
    }
}
