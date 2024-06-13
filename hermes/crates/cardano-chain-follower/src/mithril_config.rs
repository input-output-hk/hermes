//! Configuration for the Mithril Snapshot used by the follower.

use std::{
    cmp::Ordering,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use crate::Network;
use futures::future::join_all;
use tokio::{
    fs::{self, remove_file},
    io::{self, AsyncReadExt},
};
use tokio::{
    fs::{hard_link, File},
    join,
};
use tracing::{debug, error, warn};

// Mainnet Defaults.
/// Main-net Mithril Signature genesis vkey.
const DEFAULT_MAINNET_MITHRIL_GENESIS_KEY: &str = include_str!("data/mainnet-genesis.vkey");
/// Default Mithril Aggregator to use.
const DEFAULT_MAINNET_MITHRIL_AGGREGATOR: &str =
    "https://aggregator.release-mainnet.api.mithril.network/aggregator";

// Preprod Defaults
/// Preprod network Mithril Signature genesis vkey.
const DEFAULT_PREPROD_MITHRIL_GENESIS_KEY: &str = include_str!("data/preprod-genesis.vkey");
/// Default Mithril Aggregator to use.
const DEFAULT_PREPROD_MITHRIL_AGGREGATOR: &str =
    "https://aggregator.release-preprod.api.mithril.network/aggregator";

// Preview Defaults
/// Preview network Mithril Signature genesis vkey.
const DEFAULT_PREVIEW_MITHRIL_GENESIS_KEY: &str = include_str!("data/preview-genesis.vkey");
/// Default Mithril Aggregator to use.
const DEFAULT_PREVIEW_MITHRIL_AGGREGATOR: &str =
    "https://aggregator.pre-release-preview.api.mithril.network/aggregator";

/// Default name of the executable if we can't derive it.
const DEFAULT_EXE_NAME: &str = "cardano_chain_follower";

/// ENV VAR name for the data path.
const ENVVAR_MITHRIL_DATA_PATH: &str = "MITHRIL_DATA_PATH";
/// ENV VAR name for the executable name.
const ENVVAR_MITHRIL_EXE_NAME: &str = "MITHRIL_EXE_NAME";

/// Subdirectory where we download archives.
const DL_SUBDIR: &str = "dl";

/// Subdirectory where we unpack archives temporarily.
const TMP_SUBDIR: &str = "tmp";

/// Size of the file comparison buffers used for de-duping.
/// This conversion is safe, buffer size will not be > 2^32.
#[allow(clippy::cast_possible_truncation)]
const FILE_COMPARE_BUFFER_SIZE: usize = bytesize::MIB as usize;

/// Get the default storage location for mithril snapshots.
/// Defaults to: <platform data_local_dir>/<exe name>/mithril/<network>
#[allow(dead_code)]
fn get_default_mithril_path(chain: Network) -> PathBuf {
    // Get the base path for storing Data.
    // IF the ENV var is set, use that.
    // Otherwise use the system default data path for an application.
    // All else fails default to "/var/lib"
    let mut base_path = std::env::var(ENVVAR_MITHRIL_DATA_PATH).map_or_else(
        |_| dirs::data_local_dir().unwrap_or("/var/lib".into()),
        PathBuf::from,
    );

    // Get the Executable name for the data path.
    // IF the ENV var is set, use it, otherwise try and get it from the exe itself.
    // Fallback to using a default exe name if all else fails.
    let exe_name = std::env::var(ENVVAR_MITHRIL_EXE_NAME).unwrap_or(
        std::env::current_exe()
            .unwrap_or(DEFAULT_EXE_NAME.into())
            .file_name()
            .unwrap_or(OsStr::new(DEFAULT_EXE_NAME))
            .to_string_lossy()
            .to_string(),
    );

    // <base path>/<exe name>
    base_path.push(exe_name);

    // Put everything in a `mithril` sub directory.
    base_path.push("mithril");

    // <base path>/<exe name>/<network>
    base_path.push(chain.to_string());

    debug!("DEFAULT Mithril Data Path for {} : {:?}", chain, &base_path);

    // Return the final path
    base_path
}

/// Check if the src file and tmp file are identical.
async fn compare_files(src_file: &Path, tmp_file: &Path) -> bool {
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

/// Check if `immutable_file_number` is later than the current latest snapshot.
fn is_later(latest_snapshot: &Option<(PathBuf, u64)>, immutable_file_number: u64) -> bool {
    match latest_snapshot {
        None => true,
        Some((_, cmp_file_number)) if immutable_file_number > *cmp_file_number => true,
        _ => false,
    }
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
    #[allow(dead_code)]
    pub fn default_for(chain: Network) -> Self {
        match chain {
            Network::Mainnet => Self {
                path: get_default_mithril_path(chain),
                aggregator_url: DEFAULT_MAINNET_MITHRIL_AGGREGATOR.to_string(),
                genesis_key: DEFAULT_MAINNET_MITHRIL_GENESIS_KEY.to_string(),
            },
            Network::Preview => Self {
                path: get_default_mithril_path(chain),
                aggregator_url: DEFAULT_PREVIEW_MITHRIL_AGGREGATOR.to_string(),
                genesis_key: DEFAULT_PREVIEW_MITHRIL_GENESIS_KEY.to_string(),
            },
            Network::Preprod => Self {
                path: get_default_mithril_path(chain),
                aggregator_url: DEFAULT_PREPROD_MITHRIL_AGGREGATOR.to_string(),
                genesis_key: DEFAULT_PREPROD_MITHRIL_GENESIS_KEY.to_string(),
            },
        }
    }

    /// Returns the path to Download Mithril Snapshot Archives to.
    /// Will use a path relative to mithril data path.
    #[must_use]
    #[allow(dead_code)]
    pub fn dl_path(&self) -> PathBuf {
        let mut dl_path = self.path.clone();
        dl_path.push(DL_SUBDIR);
        dl_path
    }

    /// Returns the path to Latest Mithril Snapshot Data.
    /// Will use a path relative to mithril data path.
    #[must_use]
    #[allow(dead_code)]
    pub async fn latest_snapshot_path(&self) -> Option<(PathBuf, u64)> {
        // Can we read directory entries from the base path, if not then there is no latest snapshot.
        let Ok(mut entries) = fs::read_dir(&self.path).await else {
            return None;
        };

        let mut latest_snapshot: Option<(PathBuf, u64)> = None;

        loop {
            // Get the next entry, stop on any error, or no entries left.
            let Ok(Some(entry)) = entries.next_entry().await else {
                break;
            };

            // Only care about directories.
            if entry.path().is_dir() {
                if let Ok(numeric_name) = entry.file_name().to_string_lossy().parse::<u64>() {
                    if is_later(&latest_snapshot, numeric_name) {
                        latest_snapshot = Some((entry.path(), numeric_name));
                    }
                }
            }
        }

        latest_snapshot
    }

    /// Activate the tmp mithril path to a numbered snapshot path.
    /// And then remove any left over files in download or the tmp path, or old snapshots.
    #[allow(dead_code)]
    async fn activate(&self, snapshot_number: u64) -> io::Result<()> {
        let new_path = self.numbered_path(snapshot_number);
        let latest_path = self.latest_snapshot_path().await;

        // Get the latest immutable file index or 0 if none exists.
        let latest_immutable = latest_path.clone().unwrap_or(("".into(), 0)).1;

        // The number we are trying to set is newer than the latest, so lets set it.
        let cleanup_snapshot = if latest_immutable < snapshot_number {
            match fs::rename(self.tmp_path(), new_path).await {
                Ok(()) => latest_path.clone(),
                Err(err) => {
                    error!("Failed to Promote Snapshot {snapshot_number} to latest: {err:?}");
                    Some((self.tmp_path(), snapshot_number))
                },
            }
        } else {
            warn!(
                "The path to activate {:?} was NOT the latest {:?}, so it is being removed.",
                new_path, latest_path
            );
            Some((new_path, snapshot_number))
        };

        let mut cleanup_tasks = Vec::new();

        // Clean up the tmp path, and old snapshots and download artifacts.
        match cleanup_snapshot {
            None => (),
            Some((cleanup_snapshot, _snapshot)) => {
                if cleanup_snapshot.exists() {
                    cleanup_tasks.push(fs::remove_dir_all(cleanup_snapshot.clone()));
                }
            },
        };

        // Cleanup up the Download path.
        let download = self.dl_path();
        if !download.exists() {
            cleanup_tasks.push(fs::remove_dir_all(download.clone()));
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
    #[allow(dead_code)]
    pub async fn dedup_tmp(&self, tmp_file: &Path) -> bool {
        // We don't want to deduplicate directories or symlinks (or other non-files).
        // Or files that just don't exist.
        if !tmp_file.is_file() {
            return false;
        }

        // Do we have a Mithril snapshot to deduplicate against.
        let Some((snapshot_path, _)) = self.latest_snapshot_path().await else {
            // No snapshot, so nothing to de-dup against.
            return false;
        };

        // Get the matching src file in the latest mithril snapshot to compare against.
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
    #[allow(dead_code)]
    pub fn tmp_path(&self) -> PathBuf {
        let mut snapshot_path = self.path.clone();
        snapshot_path.push(TMP_SUBDIR);
        snapshot_path
    }

    /// Returns the path to Latest Tmp Snapshot Data.
    /// Will use a path relative to mithril data path.
    #[must_use]
    #[allow(dead_code)]
    pub fn numbered_path(&self, snapshot_number: u64) -> PathBuf {
        let mut snapshot_path = self.path.clone();
        snapshot_path.push(snapshot_number.to_string());
        snapshot_path
    }
}

#[cfg(test)]
mod tests {

    use std::path::Path;

    use regex::Regex;

    use super::*;

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
            test_paths(&get_default_mithril_path(network), network, root, app, None);
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
