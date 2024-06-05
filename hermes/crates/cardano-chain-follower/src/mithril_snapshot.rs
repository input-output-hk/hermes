//! Internal Mithril snapshot functions.

use std::path::PathBuf;

use dashmap::DashMap;
use once_cell::sync::Lazy;
use pallas::network::miniprotocols::Point;
use pallas_hardano::storage::immutable::FallibleBlock;

use crate::{Error, FollowerConfig, MultiEraBlockData, Network, Result};

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

/// Configured and validated path for a snapshot of a particular network.
/// It is INVALID for a network to share paths with another.
static SNAPSHOT_PATHS: Lazy<DashMap<Network, PathBuf>> = Lazy::new(DashMap::new);

/// Configured Aggregator for a network.
// static AGGREGATOR_URL: Lazy<DashMap<Network, String>> = Lazy::new(DashMap::new);

/// Configured VKEY for a network.
// static GENESIS_VKEYS: Lazy<DashMap<Network, String>> = Lazy::new(DashMap::new);

/// Current TIP of a network.
static CURRENT_TIPS: Lazy<DashMap<Network, Point>> = Lazy::new(DashMap::new);

/// Handle to the mithril sync thread.
// static SYNC_HANDLE_MAP: Lazy<DashMap<Network, String>> = Lazy::new(DashMap::new);

/// Read the current mithril path for a network.
fn read_mithril_path(network: Network) -> Option<PathBuf> {
    SNAPSHOT_PATHS
        .get(&network)
        .map(|entry| entry.value().clone())
}

/// Check that a given mithril path is not already configured.
fn check_mithril_path_conflicts(network: Network, path: PathBuf) -> Result<()> {
    if let Some(entry) = SNAPSHOT_PATHS
        .iter()
        .filter(|entry| *entry.key() != network)
        .find(|entry| *entry.value() == *path)
    {
        return Err(Error::MithrilSnapshotDirectoryAlreadyConfiguredForNetwork(
            path,
            *entry.key(),
        ));
    }
    Ok(())
}

/// Check if the configured path is a valid directory, and that it does not exist already
/// in the map.
fn set_snapshot_path(network: Network, path: PathBuf) -> Result<()> {
    if !path.is_dir() {
        return Err(Error::MithrilSnapshotDirectoryNotFound(
            path.display().to_string(),
        ));
    }

    let current_path = read_mithril_path(network);
    match current_path {
        Some(current_path) => {
            if current_path != *path {
                return Err(Error::MithrilSnapshotDirectoryAlreadyConfigured(
                    path,
                    current_path,
                ));
            }
            return Ok(());
        },
        None => {
            // Check that path isn't in any other key
            check_mithril_path_conflicts(network, path)?;
        },
    }

    Ok(())
}

/// Holds information about a Mithril snapshot.
#[derive(Clone)]
pub(crate) struct MithrilSnapshot;

impl MithrilSnapshot {
    /// Initialize Mithril snapshot processing for a particular configured network.
    pub fn init(follower_cfg: FollowerConfig) -> Result<()> {
        // Validate and Set the snapshot path configuration
        if let Some(path) = follower_cfg.mithril_snapshot_path {
            set_snapshot_path(follower_cfg.chain, path)?;
        } else {
            if follower_cfg.mithril_update {
                return Err(Error::MithrilSnapshotDirectoryNotConfigured);
            }
            return Ok(());
        }
        // Set the aggregator if not already set.

        // Set the Genesis VKEY if not already set.

        // If we want to auto-update, AND we haven't already started updating, start it.
        if follower_cfg.mithril_update {
            // TODO: Start the updater task.
        }

        Ok(())
    }

    /// Get the current tip of the configured Mithril Network
    pub fn tip(network: Network) -> Option<Point> {
        CURRENT_TIPS
            .get(&network)
            .map(|entry| entry.value().clone())
    }

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
    // pub fn from_path(path: PathBuf) -> Result<Self> {
    //    let tip = pallas_hardano::storage::immutable::get_tip(&path)
    //        .map_err(|_| Error::MithrilSnapshot)?
    //        .ok_or(Error::MithrilSnapshot)?;
    //    Ok(Self { path, tip })
    //}

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
    pub fn try_read_block(network: Network, point: Point) -> Result<Option<MultiEraBlockData>> {
        if let Some(mithril_path) = read_mithril_path(network) {
            if !MithrilSnapshot::contains_point(network, &point) {
                return Ok(None);
            }

            let mut block_data_iter =
                pallas_hardano::storage::immutable::read_blocks_from_point(&mithril_path, point)
                    .map_err(|_| Error::MithrilSnapshot)?;

            match block_data_iter.next() {
                Some(res) => {
                    let block_data = res.map_err(|_| Error::MithrilSnapshot)?;

                    Ok(Some(MultiEraBlockData(block_data)))
                },
                None => Ok(None),
            }
        } else {
            Ok(None)
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
    pub fn try_read_block_range(
        network: Network, from: Point, to: &Point,
    ) -> Result<Option<(Point, Vec<MultiEraBlockData>)>> {
        if let Some(mithril_path) = read_mithril_path(network) {
            if !MithrilSnapshot::contains_point(network, &from) {
                return Ok(None);
            }

            let blocks_iter =
                pallas_hardano::storage::immutable::read_blocks_from_point(&mithril_path, from)
                    .map_err(|_| Error::MithrilSnapshot)?;

            let mut block_data_vec = Vec::new();
            for result in blocks_iter {
                let block_data = MultiEraBlockData(result.map_err(|_| Error::MithrilSnapshot)?);

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
    /// Returns None if its not possible to iterate a mithril snapshot from the requested
    /// point for ANY reason.
    pub fn try_read_blocks_from_point(
        network: Network, point: Point,
    ) -> Option<MithrilSnapshotIterator> {
        if let Some(mithril_path) = read_mithril_path(network) {
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
        }
    }

    /// Naively checks if the snapshot contains a point.
    ///
    /// # Arguments
    ///
    /// * `point`: Point to check.
    pub fn contains_point(network: Network, point: &Point) -> bool {
        if let Some(tip) = MithrilSnapshot::tip(network) {
            point.slot_or_default() <= tip.slot_or_default()
        } else {
            false
        }
    }
}
