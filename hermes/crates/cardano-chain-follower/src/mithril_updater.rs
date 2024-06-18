//! Internal Mithril snapshot functions.

use std::{path::PathBuf, sync::Arc};

use crate::{
    error::{Error, Result},
    multi_era_block_data::MultiEraBlockData,
    network::Network,
};

use dashmap::DashMap;
use once_cell::sync::Lazy;
use pallas::network::miniprotocols::Point;
use pallas_hardano::storage::immutable::FallibleBlock;
use tokio::{sync::Mutex, task::JoinHandle};
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

/// Configured and validated path for a snapshot of a particular network.
/// It is INVALID for a network to share paths with another.
static SNAPSHOT_PATHS: Lazy<DashMap<Network, PathBuf>> = Lazy::new(DashMap::new);

/// Configured Aggregator for a network.
static AGGREGATOR_URL: Lazy<DashMap<Network, String>> = Lazy::new(DashMap::new);

/// Configured VKEY for a network.
static GENESIS_VKEYS: Lazy<DashMap<Network, String>> = Lazy::new(DashMap::new);

/// Current TIP of a network.
static CURRENT_TIPS: Lazy<DashMap<Network, Point>> = Lazy::new(DashMap::new);

/// Type we use to manage the Sync Task handle map.
type SyncMap = Arc<Mutex<DashMap<Network, JoinHandle<()>>>>;
/// Handle to the mithril sync thread.
static SYNC_HANDLE_MAP: Lazy<SyncMap> = Lazy::new(|| Arc::new(Mutex::new(DashMap::new())));

/// Check if a given value already exists in another key in the same Dashmap
/// Returns the first network found that conflicts.
fn check_map_conflicts<T: std::cmp::PartialEq + Clone>(
    network: Network, map: &DashMap<Network, T>, value: &T,
) -> Option<Network> {
    if let Some(entry) = map
        .iter()
        .filter(|entry| *entry.key() != network)
        .find(|entry| *entry.value() == *value)
    {
        return Some(*entry.key());
    }

    None
}

/// Read the current mithril path for a network.
/// This is the entire mithril snapshot, not just the immutable data.
pub(crate) fn read_mithril_path(network: Network) -> Option<PathBuf> {
    SNAPSHOT_PATHS
        .get(&network)
        .map(|entry| entry.value().clone())
}

/// Get the path of the immutable data within a mithril snapshot.
pub(crate) fn read_mithril_immutable_path(network: Network) -> Option<PathBuf> {
    match read_mithril_path(network) {
        Some(path) => {
            let mut immutable = path.clone();
            immutable.push("immutable");
            Some(immutable)
        },
        None => None,
    }
}

/// Check that a given mithril path is not already configured.
fn check_mithril_path_conflicts(network: Network, path: &PathBuf) -> Result<()> {
    if let Some(entry) = check_map_conflicts::<PathBuf>(network, &SNAPSHOT_PATHS, path) {
        return Err(Error::MithrilSnapshotDirectoryAlreadyConfiguredForNetwork(
            path.clone(),
            entry,
        ));
    }
    Ok(())
}

/// Read the current aggregator URL for a network.
pub(crate) fn read_aggregator_url(network: Network) -> Option<String> {
    AGGREGATOR_URL
        .get(&network)
        .map(|entry| entry.value().clone())
}

/// Check that a given mithril path is not already configured.
fn check_aggregator_conflicts(network: Network, aggregator_url: &String) -> Result<()> {
    if let Some(entry) = check_map_conflicts::<String>(network, &AGGREGATOR_URL, aggregator_url) {
        return Err(Error::MithrilAggregatorURLAlreadyConfiguredForNetwork(
            aggregator_url.clone(),
            entry,
        ));
    }
    Ok(())
}

/// Read the current mithril genesis vkey for a network.
pub(crate) fn read_genesis_vkey(network: Network) -> Option<String> {
    GENESIS_VKEYS
        .get(&network)
        .map(|entry| entry.value().clone())
}

/// Set the Aggregator for a particular network.
/// Aggregator is validated to ensure it has some chance of success.
async fn set_aggregator(
    network: Network, aggregator_url: String, genesis_vkey: String,
) -> Result<()> {
    // Check if the aggregator is already defined on this network to the same URL.
    if let Some(current_aggregator_url) = read_aggregator_url(network) {
        if current_aggregator_url != aggregator_url {
            return Err(Error::MithrilAggregatorURLAlreadyConfigured(
                aggregator_url,
                current_aggregator_url,
            ));
        }
    }

    // Check if this aggregator is already in-use on another network.
    check_aggregator_conflicts(network, &aggregator_url)?;

    // Not configured already, and not already in use, so make sure its valid.
    // We do this by trying to use it to get a list of snapshots.
    let client = mithril_client::ClientBuilder::aggregator(&aggregator_url, &genesis_vkey)
        .build()
        .map_err(|e| Error::MithrilClient(network, aggregator_url.clone(), e))?;

    let snapshots = client
        .snapshot()
        .list()
        .await
        .map_err(|e| Error::MithrilClient(network, aggregator_url.clone(), e))?;

    // Check we have a snapshot, and its for our network.
    match snapshots.first() {
        Some(snapshot) => {
            if snapshot.beacon.network != network.to_string() {
                return Err(Error::MithrilClientNetworkMismatch(
                    network,
                    snapshot.beacon.network.clone(),
                ));
            }
        },
        None => return Err(Error::MithrilClientNoSnapshots(network, aggregator_url)),
    }

    // Aggregator not yet configured, and works as expected, so record it.
    AGGREGATOR_URL.insert(network, aggregator_url);

    Ok(())
}

/// Try and update the current tip from an existing snapshot.
pub(crate) fn update_tip(network: Network) -> Result<()> {
    if let Some(snapshot_path) = read_mithril_immutable_path(network) {
        debug!("Updating TIP from Immutable storage @ {snapshot_path:?}");
        // If the TIP is not set, try and set it in-case there is already a snapshot in the
        // snapshot directory.
        let Some(tip) = pallas_hardano::storage::immutable::get_tip(&snapshot_path)
            .map_err(|error| Error::MithrilSnapshot(Some(error)))?
        else {
            return Err(Error::MithrilSnapshot(None));
        };

        CURRENT_TIPS.insert(network, tip);
    }

    Ok(())
}

/// Holds information about a Mithril snapshot.
#[derive(Clone)]
pub(crate) struct MithrilSnapshot;

impl MithrilSnapshot {
    /// Get the current tip of the configured Mithril Network
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
    pub fn try_read_block(network: Network, point: Point) -> Result<Option<MultiEraBlockData>> {
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
        }
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
    pub fn contains_point(network: Network, point: &Point) -> bool {
        if let Some(tip) = MithrilSnapshot::tip(network) {
            point.slot_or_default() <= tip.slot_or_default()
        } else {
            false
        }
    }
}

/*
    /// Initialize Mithril snapshot processing for a particular configured network.
    pub async fn init(follower_cfg: FollowerConfig) -> Result<()> {

        // Set the current TIP of the Mithril Snapshot, IF it already exists.
        // We don't care if this errors, its optimistic.
        debug!(
            "Updating TIP of any existing Mithril Snapshot (May Fail) for {}",
            follower_cfg.chain
        );

        let tip_status = update_tip(follower_cfg.chain);

        match tip_status {
            Ok(()) => debug!("Mithril TIP Exists and updated OK."),
            Err(e) => warn!("Mithril TIP failed to update with error: {e:?}"),
        }

            // Start the update - IFF its not already running.
            let sync_map = SYNC_HANDLE_MAP.lock().await;

            if !sync_map.contains_key(&follower_cfg.chain) {
                debug!("Mithril Autoupdate for {} : Starting", follower_cfg.chain);
                let handle = tokio::spawn(background_mithril_update(
                    follower_cfg.chain,
                    aggregator_url,
                    genesis_vkey,
                    mithril_path,
                sync_map.insert(follower_cfg.chain, handle);
                debug!("Mithril Autoupdate for {} : Started", follower_cfg.chain);
            }

            drop(sync_map);
        }

        Ok(())
    }
*/

#[cfg(test)]
mod tests {}
