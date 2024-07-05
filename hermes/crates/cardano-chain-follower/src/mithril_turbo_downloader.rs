//! Turbo Downloads for Mithril Snapshots.

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::Mutex,
};

use anyhow::{anyhow, bail, Context};
use async_compression::tokio::bufread::ZstdDecoder;
use async_trait::async_trait;
use mithril_client::{
    common::CompressionAlgorithm, snapshot_downloader::SnapshotDownloader, MithrilResult,
};
use tokio::{
    fs::{create_dir_all, File},
    io::BufReader,
    process::Command,
};
use tokio_stream::StreamExt;
use tokio_tar::Archive;
use tokio_util::codec::{FramedRead, LinesCodec};
use tracing::debug;

use crate::{
    mithril_snapshot_config::{async_hash_single_file, MithrilSnapshotConfig},
    mithril_snapshot_data::{latest_mithril_snapshot_data, FileHashMap},
    stats::{self},
};

/// A snapshot downloader that accelerates Download using `aria2`.
pub struct MithrilTurboDownloader {
    /// Handle to a HTTP client to use for downloading simply.
    http_client: reqwest::Client,
    /// Configuration for the snapshot sync.
    cfg: MithrilSnapshotConfig,
    /// Last hashmap from the previous download
    previous_hashmap: Mutex<Option<FileHashMap>>,
}

impl MithrilTurboDownloader {
    /// Constructs a new `HttpSnapshotDownloader`.
    pub fn new(cfg: MithrilSnapshotConfig) -> MithrilResult<Self> {
        let http_client = reqwest::ClientBuilder::new()
            .build()
            .with_context(|| "Building http client for TurboSnapshotDownloader failed")?;

        Ok(Self {
            http_client,
            cfg,
            previous_hashmap: Mutex::new(Option::default()),
        })
    }

    /// Take the hashmap for the previous download.  Can only be done Once.
    pub fn take_previous_hashmap(&self) -> Option<FileHashMap> {
        match self.previous_hashmap.lock() {
            Ok(mut map) => map.take(),
            Err(_) => None,
        }
    }

    /// Set the hashmap for the previous download.
    fn set_hashmap(&self, map: FileHashMap) {
        if let Ok(mut hashmap) = self.previous_hashmap.lock() {
            *hashmap = Some(map);
        }
    }
}

/// Use a consistent name for a download archive to simplify processing.
const DOWNLOAD_FILE_NAME: &str = "latest-mithril.tar.zst";

/// Download a file using `aria2` tool, with maximum number of simultaneous connections.
async fn aria2_download(dest: &Path, url: &str) -> MithrilResult<()> {
    let dest = format!("--dir={}", dest.to_string_lossy());
    let dest_file = format!("--out={DOWNLOAD_FILE_NAME}");

    let mut process = Command::new("aria2c")
        .args(["-x", "16", "-s", "16", &dest, &dest_file, url])
        .kill_on_drop(true)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let Some(stdout) = process.stdout.take() else {
        bail!("aria2c stdout channel was not readable.");
    };
    let Some(stderr) = process.stderr.take() else {
        bail!("aria2c stdout channel was not readable.");
    };

    let stdout = FramedRead::with_capacity(stdout, LinesCodec::new(), 32)
        .map(std::result::Result::unwrap_or_default);

    let stderr = FramedRead::with_capacity(stderr, LinesCodec::new(), 32)
        .map(std::result::Result::unwrap_or_default);

    let mut stream = stdout.chain(stderr);

    while let Some(msg) = stream.next().await {
        debug!("{:?}", msg);
    }

    // wait for the process to complete
    let result = process.wait().await?;
    if !result.success() {
        bail!("aria2c exited with error code {}", result);
    }

    Ok(())
}

/// Get the size of a particular file.  None = failed to get size.
async fn get_file_size(file: PathBuf) -> Option<u64> {
    let result = tokio::task::spawn_blocking(move || {
        let Ok(metadata) = file.metadata() else {
            return None;
        };
        Some(metadata.len())
    })
    .await;

    if let Ok(size) = result {
        size
    } else {
        None
    }
}

#[async_trait]
impl SnapshotDownloader for MithrilTurboDownloader {
    async fn download_unpack(
        &self, location: &str, target_dir: &Path, _compression_algorithm: CompressionAlgorithm,
        _download_id: &str, _snapshot_size: u64,
    ) -> MithrilResult<()> {
        let hashmap = FileHashMap::new();

        if let Err(error) = create_dir_all(self.cfg.dl_path()).await {
            let msg = format!(
                "Download directory {} could not be created: {}",
                self.cfg.dl_path().to_string_lossy(),
                error
            );
            Err(anyhow!(msg.clone()).context(msg))?;
        }

        if let Err(error) = create_dir_all(target_dir).await {
            let msg = format!(
                "Target directory {} could not be created: {}",
                target_dir.to_string_lossy(),
                error
            );
            Err(anyhow!(msg.clone()).context(msg))?;
        }

        debug!("Download and Unpack started='{location}' to '{target_dir:?}'.");

        stats::mithril_dl_started(self.cfg.chain);

        // First Download the Archive using Aria2 to the `dl` directory.
        // TODO(SJ): Using `aria2` as a convenience, need to change to a rust native
        // multi-connection download crate, which needs to be written.
        if let Err(error) = aria2_download(&self.cfg.dl_path(), location).await {
            // Record failed download stats
            stats::mithril_dl_finished(self.cfg.chain, None);
            return Err(error);
        }

        let mut dst_archive = self.cfg.dl_path();
        dst_archive.push(DOWNLOAD_FILE_NAME);

        // Record successful download stats
        stats::mithril_dl_finished(self.cfg.chain, get_file_size(dst_archive.clone()).await);

        // Decompress and extract and de-dupe each file in the archive.
        stats::mithril_extract_started(self.cfg.chain);

        debug!(
            "Unpacking and extracting '{}' to '{}'.",
            dst_archive.to_string_lossy(),
            target_dir.to_string_lossy()
        );

        let mut archive = Archive::new(ZstdDecoder::new(BufReader::new(
            File::open(dst_archive).await?,
        )));

        debug!("Extracting files from compressed archive.");

        let mut entries = archive.entries()?;
        let tmp_dir = self.cfg.tmp_path();
        let latest_snapshot = latest_mithril_snapshot_data(self.cfg.chain);

        let mut new_files: u64 = 0;
        let mut changed_files: u64 = 0;
        let mut total_files: u64 = 0;
        let mut extract_size: u64 = 0;
        let mut deduplicated_size: u64 = 0;

        // let latest_snapshot = self.cfg.latest_snapshot_id().await;
        while let Some(file) = entries.next().await {
            let mut file = file?;

            // Unpack the raw file first.
            file.unpack_in(tmp_dir.clone()).await?;

            if file.path()?.is_dir() {
                continue;
            }

            let relative_file = file.path()?.to_path_buf();
            let mut abs_file = tmp_dir.clone();
            abs_file.push(relative_file.clone());

            // We ONLY de-dupe files.
            if !abs_file.is_file() {
                continue;
            }

            total_files += 1;

            let file_size = get_file_size(abs_file.clone()).await.unwrap_or(0);
            extract_size += file_size;

            // Hash the new file.
            if let Some(file_hash) = async_hash_single_file(&abs_file).await {
                let _unused = hashmap.insert(relative_file.clone(), file_hash);
                // Now attempt to dedup it with the current snapshot.
                if latest_snapshot.exists() {
                    if let Some(latest_hash) = latest_snapshot.current_hash(&relative_file) {
                        if file_hash == latest_hash {
                            self.cfg.dedup_tmp(&abs_file, &latest_snapshot).await;
                        } else {
                            debug!("Changed File '{}'.", relative_file.to_string_lossy());
                            changed_files += 1;
                            deduplicated_size += file_size;
                        }
                    } else {
                        // debug!("New File '{}'.", relative_file.to_string_lossy());
                        new_files += 1;
                        deduplicated_size += file_size;
                    }
                }
            }
        }

        // Set the hashmap of the previous download.
        debug!("Hashmap entries = {}", hashmap.len());
        self.set_hashmap(hashmap);

        stats::mithril_extract_finished(
            self.cfg.chain,
            Some(extract_size),
            deduplicated_size,
            total_files - (changed_files + new_files),
            changed_files,
            new_files,
        );

        debug!("Download and Unpack finished='{location}' to '{target_dir:?}'.");

        Ok(())
    }

    async fn probe(&self, location: &str) -> MithrilResult<()> {
        debug!("HEAD Snapshot location='{location}'.");

        let request_builder = self.http_client.head(location);
        let response = request_builder.send().await.with_context(|| {
            format!("Cannot perform a HEAD for snapshot at location='{location}'")
        })?;

        let status = response.status();

        debug!("Probe for '{location}' completed: {status}");

        match response.status() {
            reqwest::StatusCode::OK => Ok(()),
            reqwest::StatusCode::NOT_FOUND => {
                Err(anyhow!("Snapshot location='{location} not found"))
            },
            status_code => Err(anyhow!("Unhandled error {status_code}")),
        }
    }
}
