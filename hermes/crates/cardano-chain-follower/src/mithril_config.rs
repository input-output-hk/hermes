//! Configuration for the Mithril Snapshot used by the follower.

use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    error::{Error, Result},
    mithril_snapshot_downloader::background_mithril_update,
    network::Network,
    snapshot_id::SnapshotId,
};

use dashmap::DashMap;
use futures::future::join_all;
use once_cell::sync::Lazy;
use pallas_hardano::storage::immutable::Point;
use tokio::{
    fs::{self, remove_file},
    fs::{hard_link, File},
    io::{self, AsyncReadExt},
    join,
    sync::{mpsc, Mutex},
    task::JoinHandle,
};
use tracing::{debug, error};

/// Type we use to manage the Sync Task handle map.
type SyncMap = Arc<Mutex<DashMap<Network, JoinHandle<()>>>>;
/// Handle to the mithril sync thread.
static SYNC_HANDLE_MAP: Lazy<SyncMap> = Lazy::new(|| Arc::new(Mutex::new(DashMap::new())));

// Mainnet Defaults.

/// Subdirectory where we download archives.
const DL_SUBDIR: &str = "dl";

/// Subdirectory where we unpack archives temporarily.
const TMP_SUBDIR: &str = "tmp";

/// Size of the file comparison buffers used for de-duping.
/// This conversion is safe, buffer size will not be > 2^32.
#[allow(clippy::cast_possible_truncation)]
const FILE_COMPARE_BUFFER_SIZE: usize = bytesize::MIB as usize;

/// Check if the src file and tmp file are identical.
async fn compare_files(src_file: &Path, tmp_file: &Path) -> bool {
    // Get data describing both files.
    let Ok(src_file_data) = src_file.metadata() else {
        return false;
    };
    let Ok(tmp_file_data) = tmp_file.metadata() else {
        return false;
    };

    // if they are different sizes, no way we can de-duplicate them.
    if src_file_data.len() != tmp_file_data.len() {
        return false;
    }

    // Finally ensure they are identical data
    let mut src = match File::open(src_file).await {
        Ok(f) => f,
        Err(err) => {
            error!(
                "Failed to open src {} for de-duping: {}",
                src_file.to_string_lossy(),
                err
            );
            return false;
        },
    };

    let mut tmp = match File::open(tmp_file).await {
        Ok(f) => f,
        Err(err) => {
            error!(
                "Failed to open tmp {} for de-duping: {}",
                tmp_file.to_string_lossy(),
                err
            );
            return false;
        },
    };

    let mut src_buffer = vec![0; FILE_COMPARE_BUFFER_SIZE].into_boxed_slice();
    let mut tmp_buffer = vec![0; FILE_COMPARE_BUFFER_SIZE].into_boxed_slice();

    loop {
        let (src_res, tmp_res) =
            join!(src.read(&mut src_buffer[..]), tmp.read(&mut tmp_buffer[..]));
        // Any IO error means we can not dedup the files. (We shouldn't get them...)
        let src_size = match src_res {
            Ok(size) => size,
            Err(err) => {
                error!(
                    "IO Error de-duping {} : {}",
                    src_file.to_string_lossy(),
                    err
                );
                return false;
            },
        };
        let tmp_size = match tmp_res {
            Ok(size) => size,
            Err(err) => {
                error!(
                    "IO Error de-duping {} : {}",
                    tmp_file.to_string_lossy(),
                    err
                );
                return false;
            },
        };

        // Not the same size read from file (Shouldn't happen but check anyway).
        if src_size != tmp_size {
            error!("Size of buffered data src {src_size} and tmp {tmp_size} do not match",);
            return false;
        }

        // Read 0 bytes, then we are finished.
        if src_size == 0 {
            break;
        }

        let Some(src_cmp) = src_buffer.get(..src_size) else {
            error!("Unreachable, Can't read more than the src buffer size ???");
            return false;
        };
        let Some(tmp_cmp) = tmp_buffer.get(..tmp_size) else {
            error!("Unreachable, Can't read more than the tmp buffer size ???");
            return false;
        };

        // Compare the two buffers, if they differ we will not dedup.
        if src_cmp.cmp(tmp_cmp) != Ordering::Equal {
            return false;
        }
    }

    true
}

/// Configuration used for the Mithril Snapshot downloader.
#[derive(Clone, Debug)]
pub struct MithrilSnapshotConfig {
    /// Path to the Mithril snapshot the follower should use.
    /// Note: this is a base directory.  The Actual data will be stored under here.
    /// archive downloads -> `<mithril_snapshot_path>/dl`
    /// unpacked snapshots -> `<mithril_snapshot_path>/<immutable-file-no>`
    /// extracting snapshots -> `<mithril_snapshot_path>/tmp`
    pub path: PathBuf,
    /// Address of the Mithril Aggregator to use to find the latest snapshot data to
    /// download.
    pub aggregator_url: String,
    /// The Genesis Key needed for a network to do Mithril snapshot validation.
    pub genesis_key: String,
}

impl MithrilSnapshotConfig {
    /// Sets the defaults for a given cardano network.
    /// Each network has a different set of defaults, so no single "default" can apply.
    /// This function is preferred to the `default()` standard function.
    #[must_use]
    pub fn default_for(chain: Network) -> Self {
        Self {
            path: chain.default_mithril_path(),
            aggregator_url: chain.default_mithril_aggregator(),
            genesis_key: chain.default_mithril_genesis_key(),
        }
    }

    /// Returns the path to Download Mithril Snapshot Archives to.
    /// Will use a path relative to mithril data path.
    #[must_use]
    pub(crate) fn dl_path(&self) -> PathBuf {
        let mut dl_path = self.path.clone();
        dl_path.push(DL_SUBDIR);
        dl_path
    }

    /// Returns the path to Latest Mithril Snapshot Data.
    /// Will use a path relative to mithril data path.
    #[must_use]
    pub(crate) async fn latest_snapshot_path(&self) -> Option<SnapshotId> {
        // Can we read directory entries from the base path, if not then there is no latest snapshot.
        let Ok(mut entries) = fs::read_dir(&self.path).await else {
            return None;
        };

        let mut latest_snapshot: Option<SnapshotId> = None;

        loop {
            // Get the next entry, stop on any error, or no entries left.
            let Ok(Some(entry)) = entries.next_entry().await else {
                break;
            };

            if let Some(snapshot) = SnapshotId::try_new(&entry.path()) {
                if snapshot > latest_snapshot {
                    // Found a new latest snapshot
                    latest_snapshot = Some(snapshot);
                }
            }
        }

        latest_snapshot
    }

    /// Activate the tmp mithril path to a numbered snapshot path.
    /// And then remove any left over files in download or the tmp path, or old snapshots.
    pub(crate) async fn activate(&self, snapshot_number: u64) -> io::Result<PathBuf> {
        let new_path = self.mithril_path(snapshot_number);
        let latest_path = self.latest_snapshot_path().await;

        // Can't activate anything if the tmp directory does not exist.
        if !self.tmp_path().is_dir() {
            error!("No tmp path found to activate.");
            return Err(io::Error::new(io::ErrorKind::NotFound, "No tmp path found"));
        }

        // Check if we would actually be making a newer snapshot active. (Should never fail, but check anyway.)
        if let Some(latest) = latest_path {
            if latest >= snapshot_number {
                error!(
                    "Latest snapshot {latest:?} is >= than requested snapshot {snapshot_number}"
                );
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Latest snapshot is newer or equal",
                ));
            }
        };

        // Rename the tmp path to the new numbered path.
        fs::rename(self.tmp_path(), &new_path).await?;

        // Cleanup older snapshots, dl and tmp directories if they exist.
        self.cleanup().await?;

        Ok(new_path)
    }

    /// Cleanup the tmp mithril path, all old mithril paths and the dl path.
    /// Removes those directories if they exist and all the files they contain.
    async fn cleanup(&self) -> io::Result<()> {
        let mut cleanup_tasks = Vec::new();

        // Cleanup up the Download path. (Finished with the archive)
        let download = self.dl_path();
        if !download.exists() {
            cleanup_tasks.push(fs::remove_dir_all(download.clone()));
        }

        // Cleanup up the tmp path. (Shouldn't normally exist, but clean it anyway)
        let tmp = self.tmp_path();
        if !tmp.exists() {
            cleanup_tasks.push(fs::remove_dir_all(tmp.clone()));
        }

        // Cleanup all numbered paths which are not this latest path
        match fs::read_dir(&self.path).await {
            Err(err) => error!(
                "Unexpected failure reading entries in the mithril path {} : {}",
                self.path.to_string_lossy(),
                err
            ),
            Ok(mut entries) => {
                // Get latest mithril snapshot path and number.
                let latest_snapshot = self.latest_snapshot_path().await;

                loop {
                    // Get the next entry, stop on any error, or no entries left.
                    let Ok(Some(entry)) = entries.next_entry().await else {
                        break;
                    };

                    // If None, its not a snapshot path, so continue.
                    if let Some(this_snapshot) = SnapshotId::try_new(&entry.path()) {
                        // Don't do anything with the latest snapshot.
                        if this_snapshot != latest_snapshot {
                            cleanup_tasks.push(fs::remove_dir_all(entry.path()));
                        }
                    };
                }
            },
        }

        for result in join_all(cleanup_tasks).await {
            match result {
                Ok(()) => (),
                Err(err) => {
                    error!("Failed to cleanup snapshot:  {err:?}");
                },
            }
        }

        Ok(())
    }

    /// Deduplicate a file in the tmp directory vs its equivalent in the current snapshot.
    ///
    /// Files are first compared for binary equivalence.
    /// If they are identical, the `tmp` file is removed and replaced with a hard-link to the file in
    /// the latest snapshot.
    ///
    /// Returns true if de-duped, false otherwise.
    pub(crate) async fn dedup_tmp(&self, tmp_file: &Path) -> bool {
        // We don't want to deduplicate directories or symlinks (or other non-files).
        // Or files that just don't exist.
        if !tmp_file.is_file() {
            return false;
        }

        // Do we have a Mithril snapshot to deduplicate against.
        let Some(latest_snapshot) = self.latest_snapshot_path().await else {
            // No snapshot, so nothing to de-dup against.
            return false;
        };

        // Get the matching src file in the latest mithril snapshot to compare against.
        let snapshot_path = latest_snapshot.as_ref();
        let tmp_path = self.tmp_path();
        let Ok(relative_file) = tmp_file.strip_prefix(tmp_path) else {
            return false;
        };
        let src_file = snapshot_path.join(relative_file);
        let src_file = src_file.as_path();

        // First check if we even have a snapshot_file to compare with, and that its actually just a file.
        if !src_file.is_file() {
            // No snapshot file (or not a file), so nothing to de-dup
            return false;
        }

        // Finally ensure they are identical data
        if !compare_files(src_file, tmp_file).await {
            // Not identical data, so we can't de-duplicate
            return false;
        }

        // IF we make it here, the files are identical, so we can de-dup them safely.
        // Remove the tmp file momentarily.
        if let Err(error) = remove_file(tmp_file).await {
            error!(
                "Error removing tmp file  {} :  {}",
                tmp_file.to_string_lossy(),
                error
            );
            return false;
        }

        // Hardlink the src file to the tmp file.
        if let Err(error) = hard_link(src_file, tmp_file).await {
            error!(
                "Error linking src file {} to tmp file {} : {}",
                src_file.to_string_lossy(),
                tmp_file.to_string_lossy(),
                error
            );
        }

        // And if we made it here, file was successfully de-duped.  YAY.
        true
    }

    /// Returns the path to Latest Tmp Snapshot Data.
    /// Will use a path relative to mithril data path.
    #[must_use]
    pub(crate) fn tmp_path(&self) -> PathBuf {
        let mut snapshot_path = self.path.clone();
        snapshot_path.push(TMP_SUBDIR);
        snapshot_path
    }

    /// Returns the path to the Numbered Snapshot Data.
    /// Will use a path relative to mithril data path.
    #[must_use]
    pub(crate) fn mithril_path(&self, snapshot_number: u64) -> PathBuf {
        let mut snapshot_path = self.path.clone();
        snapshot_path.push(snapshot_number.to_string());
        snapshot_path
    }

    /// Check if the Mithril Snapshot Path is valid an usable.
    async fn validate_path(&self) -> Result<()> {
        let path = self.path.clone();
        debug!("Validating Mithril Snapshot Path: {:?}", path);

        // If the path does not exist, try and make it.
        if !path.exists() {
            // Try and make the directory.
            fs::create_dir_all(&path)
                .await
                .map_err(|e| Error::MithrilSnapshotDirectoryCreation(path.clone(), e))?;
        }

        // If the path is NOT a directory, then we can't use it.
        if !path.is_dir() {
            return Err(Error::MithrilSnapshotDirectoryNotFound(
                path.display().to_string(),
            ));
        }

        // If the directory is not writable then we can't use
        if !check_writable(&path) {
            return Err(Error::MithrilSnapshotDirectoryNotWritable(path.clone()));
        }

        Ok(())
    }

    /// Validate the Genesis VKEY is at least the correct kind of data.
    fn validate_genesis_vkey(&self, chain: Network) -> Result<()> {
        // First sanitize the vkey by removing all whitespace and make sure its actually valid
        // hex.
        let vkey = remove_whitespace(&self.genesis_key);
        if !is_hex(&vkey) {
            return Err(Error::MithrilGenesisVKeyNotHex(chain));
        }

        Ok(())
    }

    /// Validate the Aggregator is resolvable and responsive.
    async fn validate_aggregator_url(&self, chain: Network) -> Result<()> {
        let url = self.aggregator_url.clone();
        let key = self.genesis_key.clone();

        debug!("Validating Aggregator URL: {:?}", url);

        // Not configured already, and not already in use, so make sure its valid.
        // We do this by trying to use it to get a list of snapshots.
        let client = mithril_client::ClientBuilder::aggregator(&url, &key)
            .build()
            .map_err(|e| Error::MithrilClient(chain, url.clone(), e))?;

        let snapshots = client
            .snapshot()
            .list()
            .await
            .map_err(|e| Error::MithrilClient(chain, url.clone(), e))?;

        // Check we have a snapshot, and its for our network.
        match snapshots.first() {
            Some(snapshot) => {
                if snapshot.beacon.network != chain.to_string() {
                    return Err(Error::MithrilClientNetworkMismatch(
                        chain,
                        snapshot.beacon.network.clone(),
                    ));
                }
            },
            None => return Err(Error::MithrilClientNoSnapshots(chain, url)),
        }

        Ok(())
    }

    /// Validate the mithril sync configuration is correct.
    pub(crate) async fn validate(&self, chain: Network) -> Result<()> {
        // Validate the path exists and is a directory, and is writable.
        self.validate_path().await?;
        // Validate the genesis vkey is valid.
        self.validate_genesis_vkey(chain)?;
        // Validate the Aggregator is valid and responsive.
        self.validate_aggregator_url(chain).await?;

        Ok(())
    }

    /// Run a Mithril Follower for the given network and configuration.
    pub(crate) async fn run(&self, chain: Network) -> Result<mpsc::Receiver<Point>> {
        debug!("Mithril Autoupdate for {} : Starting", chain);

        // Start the mITHRIL uPDATER - IFF its not already running.
        // This lock also effectively stops us starting multiple updaters for multiple networks simultaneously.
        // They will be started one at a time.
        let sync_map = SYNC_HANDLE_MAP.lock().await;

        // If we already have an entry in this map, then we are already running.
        if sync_map.contains_key(&chain) {
            error!("Mithril Autoupdate already running for {}", chain);
            return Err(Error::MithrilSnapshotUpdaterAlreadyRunning(chain));
        }

        self.validate(chain).await?;

        // Create a Queue we use to signal the Live Blockchain Follower that the Mithril Snapshot TIP has changed.
        let (tx, rx) = mpsc::channel::<Point>(2);

        let handle = tokio::spawn(background_mithril_update(chain, self.clone(), tx));
        sync_map.insert(chain, handle);
        debug!("Mithril Autoupdate for {} : Started", chain);

        drop(sync_map);

        Ok(rx)
    }
}

/// Check that a given mithril snapshot path and everything in it is writable.
/// We don't care why its NOT writable, just that it is either all writable, or not.
/// Will return false on the first detection of a read only file or directory.
fn check_writable(path: &Path) -> bool {
    // Check the permissions of the current path
    if let Ok(metadata) = path.metadata() {
        if metadata.permissions().readonly() {
            return false;
        }
    }

    // Can't read the directory for any reason, so can't write to the directory.
    let path_iterator = match path.read_dir() {
        Err(_) => return false,
        Ok(entries) => entries,
    };

    // Recursively check the contents of the directory
    for entry in path_iterator {
        let Ok(entry) = entry else { return false };

        // If the entry is a directory, recursively check its permissions
        // otherwise just check we could re-write it.
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_dir() {
                // This can NOT be combined with the `if` above.
                // Doing so will cause the `else` to run on non-writable directories.
                // Which is wrong.
                if !check_writable(&entry.path()) {
                    return false;
                }
            } else {
                // If its not a directory then it must be a file.
                if metadata.permissions().readonly() {
                    return false;
                }
            }
        } else {
            // Can't identify the file type, so we can;t dedup it.
            return false;
        }
    }
    // Otherwise we could write everything we scanned.
    true
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

#[cfg(test)]
mod tests {

    use std::path::Path;

    use super::*;
    use crate::network::{ENVVAR_MITHRIL_DATA_PATH, ENVVAR_MITHRIL_EXE_NAME};

    use regex::Regex;

    fn test_paths(
        path: &Path, network: Network, data_root: &str, exe_name: &str, subdir: Option<&str>,
    ) {
        let mut re_format: String = data_root.to_string();
        re_format += exe_name;
        re_format += r"\/mithril\/";
        re_format += &network.to_string();
        if let Some(subdir) = subdir {
            re_format += "/";
            re_format += subdir;
        }

        let re = Regex::new(&re_format).expect("Bad Regex!");
        assert!(re.is_match(&path.to_string_lossy()));
    }

    const DEFAULT_ROOT: &str = r"^\/home\/[^\/]*\/.local\/share\/";
    const DEFAULT_APP: &str = r"cardano_chain_follower-[^\/]*";

    const CUSTOM_EXE: &str = r"MyFollowerExecutable";
    const CUSTOM_ROOT: &str = r"\/var\/lib\/";

    #[cfg(target_os = "linux")]
    #[test]
    fn test_base_path() {
        fn test_network(network: Network, root: &str, app: &str) {
            test_paths(&network.default_mithril_path(), network, root, app, None);
        }
        // Use the probed EXE name
        test_network(Network::Mainnet, DEFAULT_ROOT, DEFAULT_APP);
        test_network(Network::Preview, DEFAULT_ROOT, DEFAULT_APP);
        test_network(Network::Preprod, DEFAULT_ROOT, DEFAULT_APP);

        // Now try and force the EXE Name with an env var.
        std::env::set_var(ENVVAR_MITHRIL_EXE_NAME, CUSTOM_EXE);
        test_network(Network::Mainnet, DEFAULT_ROOT, CUSTOM_EXE);
        test_network(Network::Preview, DEFAULT_ROOT, CUSTOM_EXE);
        test_network(Network::Preprod, DEFAULT_ROOT, CUSTOM_EXE);

        // Now try and force the Root path with an env var.
        std::env::set_var(ENVVAR_MITHRIL_DATA_PATH, CUSTOM_ROOT);
        test_network(Network::Mainnet, CUSTOM_ROOT, CUSTOM_EXE);
        test_network(Network::Preview, CUSTOM_ROOT, CUSTOM_EXE);
        test_network(Network::Preprod, CUSTOM_ROOT, CUSTOM_EXE);
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn test_working_paths() {
        async fn test_network(network: Network) {
            let cfg = MithrilSnapshotConfig::default_for(network);

            test_paths(
                &cfg.dl_path(),
                network,
                DEFAULT_ROOT,
                DEFAULT_APP,
                Some(DL_SUBDIR),
            );

            test_paths(
                &cfg.tmp_path(),
                network,
                DEFAULT_ROOT,
                DEFAULT_APP,
                Some(TMP_SUBDIR),
            );

            let latest = cfg.latest_snapshot_path().await;
            assert!(latest.is_none());
        }

        test_network(Network::Mainnet).await;
        test_network(Network::Preprod).await;
        test_network(Network::Preview).await;
    }
}
