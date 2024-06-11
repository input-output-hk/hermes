//! Internal Mithril snapshot functions.

use std::{fs, path::PathBuf, sync::Arc};

use dashmap::DashMap;
use once_cell::sync::Lazy;
use pallas::network::miniprotocols::Point;
use pallas_hardano::storage::immutable::FallibleBlock;
use tokio::{sync::Mutex, task::JoinHandle};
use tracing::{debug, warn};

use crate::{
    mithril_snapshot_downloader::background_mithril_update, Error, FollowerConfig,
    MultiEraBlockData, Network, Result,
};

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

/// Check that a given mithril snapshot path and everything in it is writable.
/// We don't care why its NOT writable, just that it is either all writable, or not.
/// Will return false on the first detection of a read only file or directory.
fn check_writable(path: &PathBuf) -> bool {
    // Check the permissions of the current path
    if let Ok(metadata) = fs::metadata(path) {
        if metadata.permissions().readonly() {
            return false;
        }
    }

    // Can't read the directory for any reason, so can't write to the directory.
    let path_iterator = match fs::read_dir(path) {
        Err(_) => return false,
        Ok(entries) => entries,
    };

    // Recursively check the contents of the directory
    for entry in path_iterator {
        let entry = match entry {
            Err(_) => return false,
            Ok(entry) => entry,
        };

        // If the entry is a directory, recursively check its permissions
        // otherwise just check we could re-write it.
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_dir() {
                if !check_writable(&entry.path()) {
                    return false;
                }
            } else {
                // If its not a directory then it must be a file.
                if metadata.permissions().readonly() {
                    return false;
                }
            }
        }
    }
    // Otherwise we could write everything we scanned.
    true
}

/// Check if the configured path is a valid directory, and that it does not exist already
/// in the map.
fn set_snapshot_path(network: Network, path: PathBuf, update: bool) -> Result<()> {
    // Check if we are already configured for a different path, or the path is used by a
    // different network.
    let current_path = read_mithril_path(network);
    match current_path {
        Some(current_path) => {
            if current_path != *path {
                return Err(Error::MithrilSnapshotDirectoryAlreadyConfigured(
                    path,
                    current_path,
                ));
            }
        },
        None => {
            // Check that path isn't in any other key
            check_mithril_path_conflicts(network, &path)?;
        },
    }

    // Path not in use, or its not changed, so try and set it up.

    // If the path does not exist, try and make it.
    if !path.exists() {
        // Try and make the directory.
        fs::create_dir_all(&path)
            .map_err(|e| Error::MithrilSnapshotDirectoryCreationError(path.clone(), e))?;
    }

    // If the path is NOT a directory, then we can't use it.
    if !path.is_dir() {
        return Err(Error::MithrilSnapshotDirectoryNotFound(
            path.display().to_string(),
        ));
    }

    // if we plan to update the snapshot, the directory and all its contents needs to be
    // writable. Do this test last, because it could be relatively slow, and is not
    // necessary if any of the other checks fail.
    if update {
        // If the directory is not writable then we can't use
        if !check_writable(&path) {
            return Err(Error::MithrilSnapshotDirectoryNotWritable(path.clone()));
        }
    }

    // All the previous checks passed, so we can use this path.
    // Effectively a NOOP if the path was perviously added, but it doesn't hurt to do it
    // again.
    SNAPSHOT_PATHS.insert(network, path);

    Ok(())
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
        .map_err(|e| Error::MithrilClientError(network, aggregator_url.clone(), e))?;

    let snapshots = client
        .snapshot()
        .list()
        .await
        .map_err(|e| Error::MithrilClientError(network, aggregator_url.clone(), e))?;

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
        None => {
            return Err(Error::MithrilClientNoSnapshotsError(
                network,
                aggregator_url,
            ))
        },
    }

    // Aggregator not yet configured, and works as expected, so record it.
    AGGREGATOR_URL.insert(network, aggregator_url);

    Ok(())
}

/// Remove whitespace from a string and return the new string
fn remove_whitespace(s: &str) -> String {
    s.chars()
        .filter(|&c| !c.is_ascii_whitespace())
        .collect::<String>()
}

/// Check if a string is an even number of hex digits.
fn is_hex(s: &str) -> bool {
    s.chars().count() % 2 == 0 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Set the genesis VKEY for a network, but only if its not already set, or has not
/// changed if it is.
fn set_genesis_vkey(network: Network, vkey: &str) -> Result<()> {
    // First sanitize the vkey by removing all whitespace and make sure its actually valid
    // hex.
    let vkey = remove_whitespace(vkey);
    if !is_hex(&vkey) {
        return Err(Error::MithrilGenesisVKeyNotHex(network));
    }

    // Check the Genesis VKEY is not already configured and if so, its not the same.
    if let Some(current_vkey) = read_genesis_vkey(network) {
        if current_vkey != vkey {
            return Err(Error::MithrilGenesisVKeyMismatch(network));
        }
    }

    GENESIS_VKEYS.insert(network, vkey);

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
    /// Initialize Mithril snapshot processing for a particular configured network.
    pub async fn init(follower_cfg: FollowerConfig) -> Result<()> {
        // Validate and Set the snapshot path configuration
        debug!(
            "Validating Snapshot Path: {:?}",
            follower_cfg.mithril_snapshot_path
        );
        if let Some(path) = follower_cfg.mithril_snapshot_path {
            set_snapshot_path(follower_cfg.chain, path, follower_cfg.mithril_update)?;
        } else {
            if follower_cfg.mithril_update {
                return Err(Error::MithrilSnapshotDirectoryNotConfigured);
            }
            return Ok(());
        }

        // Set the Genesis VKEY if not already set.
        debug!(
            "Validating Genesis Key: {:?}",
            follower_cfg.mithril_genesis_key
        );
        if let Some(genesis_vkey) = follower_cfg.mithril_genesis_key {
            set_genesis_vkey(follower_cfg.chain, &genesis_vkey)?;

            // Set the aggregator if not already set - will fail if Genesis key not already set
            // correctly.
            debug!(
                "Validating Aggregator URL: {:?}",
                follower_cfg.mithril_aggregator_address
            );
            if let Some(aggregator_url) = follower_cfg.mithril_aggregator_address {
                set_aggregator(follower_cfg.chain, aggregator_url, genesis_vkey).await?;
            }
        }

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

        // If we want to auto-update, AND we haven't already started updating, start it.
        // Can only start it IF the Aggregator and VKEy are configured as well.
        if follower_cfg.mithril_update {
            // All must be set or we can't start the autoupdate.
            let Some(aggregator_url) = read_aggregator_url(follower_cfg.chain) else {
                return Err(Error::MithrilUpdateRequiresAggregatorAndVkeyAndPath(
                    follower_cfg.chain,
                ));
            };
            let Some(genesis_vkey) = read_genesis_vkey(follower_cfg.chain) else {
                return Err(Error::MithrilUpdateRequiresAggregatorAndVkeyAndPath(
                    follower_cfg.chain,
                ));
            };
            let Some(mithril_path) = read_mithril_path(follower_cfg.chain) else {
                return Err(Error::MithrilUpdateRequiresAggregatorAndVkeyAndPath(
                    follower_cfg.chain,
                ));
            };

            // Start the update - IFF its not already running.
            let sync_map = SYNC_HANDLE_MAP.lock().await;

            if !sync_map.contains_key(&follower_cfg.chain) {
                debug!("Mithril Autoupdate for {} : Starting", follower_cfg.chain);
                let handle = tokio::spawn(background_mithril_update(
                    follower_cfg.chain,
                    aggregator_url,
                    genesis_vkey,
                    mithril_path,
                ));
                sync_map.insert(follower_cfg.chain, handle);
                debug!("Mithril Autoupdate for {} : Started", follower_cfg.chain);
            }

            drop(sync_map);
        }

        Ok(())
    }

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
        if let Some(mithril_path) = read_mithril_immutable_path(network) {
            if !MithrilSnapshot::contains_point(network, &from) {
                return Ok(None);
            }

            let blocks_iter =
                pallas_hardano::storage::immutable::read_blocks_from_point(&mithril_path, from)
                    .map_err(|error| Error::MithrilSnapshot(Some(error)))?;

            let mut block_data_vec = Vec::new();
            for result in blocks_iter {
                let block_data = MultiEraBlockData(result.map_err(Error::MithrilSnapshotChunk)?);

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

#[cfg(test)]
mod tests {
    use dashmap::DashMap;

    use crate::mithril_snapshot::{check_map_conflicts, Network};

    #[test]
    fn test_check_map_conflicts() {
        let network1 = Network::Mainnet;
        let network2 = Network::Testnet;

        let map: DashMap<Network, i32> = DashMap::new();

        // When map is empty there is no conflict.
        assert_eq!(None, check_map_conflicts(network1, &map, &5));

        map.insert(network1, 5);

        // When network is the same there is no conflict.
        assert_eq!(None, check_map_conflicts(network1, &map, &5));

        let conflict = check_map_conflicts(network2, &map, &5);

        // When network is different there is a conflict.
        assert_eq!(Some(Network::Mainnet), conflict);
    }
}
