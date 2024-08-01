//! Turbo Downloads for Mithril Snapshots.

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use anyhow::{anyhow, bail, Context};
use async_compression::tokio::bufread::ZstdDecoder;
use async_trait::async_trait;
use crossbeam_skiplist::SkipSet;
use fmmap::{
    tokio::{AsyncMmapFile, AsyncMmapFileExt, AsyncOptions},
    MmapFileExt,
};
use mithril_client::{
    common::CompressionAlgorithm, snapshot_downloader::SnapshotDownloader, MithrilResult,
};
use tokio::{
    fs::{create_dir_all, hard_link},
    process::Command,
    sync::mpsc::{self, UnboundedSender},
    task::JoinHandle,
};
use tokio_stream::StreamExt;
use tokio_tar::Archive;
use tokio_util::codec::{FramedRead, LinesCodec};
use tracing::{debug, error};

use crate::{
    mithril_snapshot_config::MithrilSnapshotConfig,
    mithril_snapshot_data::latest_mithril_snapshot_data,
    stats::{self},
};

/// A snapshot downloader that accelerates Download using `aria2`.
pub struct Inner {
    /// Handle to a HTTP client to use for downloading simply.
    http_client: reqwest::Client,
    /// Configuration for the snapshot sync.
    cfg: MithrilSnapshotConfig,
    /// Last hashmap/list of changed chunks from the previous download
    new_chunks: Arc<SkipSet<PathBuf>>,

    /// The number of files that were new in this download.
    new_files: AtomicU64,
    /// The number of files that changed in this download.
    chg_files: AtomicU64,
    /// The total number of files in the download.
    tot_files: AtomicU64,
    /// The total size of the files extracted in the download.
    ext_size: AtomicU64,
    /// The total size of the files we deduplicated.
    ddup_size: AtomicU64,
}

/// A snapshot downloader that accelerates Download using `aria2`.
pub struct MithrilTurboDownloader {
    /// inner arc wrapped configuration
    inner: Arc<Inner>,
}

/// This macro is what happens every time the file is different from previous.
macro_rules! changed_file {
    ($self:ident, $rel_file:ident, $abs_file:ident, $new_size:ident) => {
        $self.chg_files.fetch_add(1, Ordering::SeqCst);
        $self.ddup_size.fetch_add($new_size, Ordering::SeqCst);
        if $abs_file.extension() == Some(OsStr::new("chunk")) {
            $self.new_chunks.insert($abs_file);
        }
    };
}

/// This macro is what happens every time we decide the file can;t be deduplicated.
macro_rules! new_file {
    ($self:ident, $rel_file:ident, $abs_file:ident, $new_size:ident) => {
        $self.new_files.fetch_add(1, Ordering::SeqCst);
        $self.ddup_size.fetch_add($new_size, Ordering::SeqCst);
        if $abs_file.extension() == Some(OsStr::new("chunk")) {
            $self.new_chunks.insert($abs_file);
        }
    };
}

impl MithrilTurboDownloader {
    /// Constructs a new `HttpSnapshotDownloader`.
    pub fn new(cfg: MithrilSnapshotConfig) -> MithrilResult<Self> {
        let http_client = reqwest::ClientBuilder::new()
            .build()
            .with_context(|| "Building http client for TurboSnapshotDownloader failed")?;

        Ok(Self {
            inner: Arc::new(Inner {
                http_client,
                cfg,
                new_chunks: Arc::new(SkipSet::new()),
                new_files: AtomicU64::new(0),
                chg_files: AtomicU64::new(0),
                tot_files: AtomicU64::new(0),
                ext_size: AtomicU64::new(0),
                ddup_size: AtomicU64::new(0),
            }),
        })
    }

    /// Take the hashmap for the previous download.
    pub fn get_new_chunks(&self) -> Arc<SkipSet<PathBuf>> {
        self.inner.new_chunks.clone()
    }

    /// Create directories required to exist for download to succeed.
    async fn create_directories(&self, target_dir: &Path) -> MithrilResult<()> {
        if let Err(error) = create_dir_all(self.inner.cfg.dl_path()).await {
            let msg = format!(
                "Download directory {} could not be created: {}",
                self.inner.cfg.dl_path().to_string_lossy(),
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

        Ok(())
    }

    /// Actually download the file to a temp directory first.
    async fn dl(&self, location: &str, target_dir: &Path) -> MithrilResult<PathBuf> {
        self.create_directories(target_dir).await?;

        debug!("Download and Unpack started='{location}' to '{target_dir:?}'.");

        stats::mithril_dl_started(self.inner.cfg.chain);

        // First Download the Archive using Aria2 to the `dl` directory.
        // TODO(SJ): Using `aria2` as a convenience, need to change to a rust native
        // multi-connection download crate, which needs to be written.
        if let Err(error) = aria2_download(&self.inner.cfg.dl_path(), location).await {
            // Record failed download stats
            stats::mithril_dl_finished(self.inner.cfg.chain, None);
            return Err(error);
        }

        let mut dst_archive = self.inner.cfg.dl_path();
        dst_archive.push(DOWNLOAD_FILE_NAME);

        // Record successful download stats
        stats::mithril_dl_finished(
            self.inner.cfg.chain,
            get_file_size(dst_archive.clone()).await,
        );

        Ok(dst_archive)
    }

    /// Start the Dedup Workers.
    /// Allows us to overlap De-duping and Unpacking.
    fn start_dedup(&self) -> (UnboundedSender<PathBuf>, JoinHandle<()>) {
        // Maximum number of files pending to dedup simultaneously.
        let (tx, mut rx) = mpsc::unbounded_channel::<PathBuf>();

        let shared_self = self.inner.clone();

        let dedup_processor = tokio::spawn(async move {
            debug!(chain = %shared_self.cfg.chain, "Background DeDup : Started");

            tokio::task::spawn_blocking(move || {
                rayon::scope(|s| {
                    while let Some(dedup_this) = rx.blocking_recv() {
                        let shared_self = shared_self.clone();
                        s.spawn(move |_| {
                            let rel_file = dedup_this.clone();
                            let tmp_dir = shared_self.cfg.tmp_path();
                            let latest_snapshot =
                                latest_mithril_snapshot_data(shared_self.cfg.chain);

                            let mut abs_file = tmp_dir.clone();
                            abs_file.push(rel_file.clone());

                            let mut prev_file = latest_snapshot.id().path();
                            prev_file.push(rel_file.clone());

                            //debug!(
                            //    "Comparing File: {} with {} ",
                            //    abs_file.to_string_lossy(),
                            //    prev_file.to_string_lossy()
                            //);

                            // We ONLY de-dupe files.
                            if !abs_file.is_file() {
                                debug!("No Dedup, Not a File '{}'.", rel_file.to_string_lossy());
                                return;
                            }

                            shared_self.tot_files.fetch_add(1, Ordering::SeqCst);

                            let Ok((new_file, new_size)) = mmap_open_sync(&abs_file) else {
                                return;
                            };

                            shared_self.ext_size.fetch_add(new_size, Ordering::SeqCst);

                            // Assume that any files not in the immutable directory have changed, and don't check them
                            if !rel_file.starts_with(Path::new("immutable")) {
                                debug!(file = %rel_file.display(), "Not deduplicating: Not immutable");
                                new_file!(shared_self, rel_file, abs_file, new_size);
                                drop(new_file);
                                return;
                            }

                            // If the previous file doesn't exist, or isn't a file, then the file is new.
                            if !prev_file.is_file() {
                                // debug!(file = %prev_file.display(), "Not deduplicating: Previous File does not exist or is not a file.");
                                new_file!(shared_self, rel_file, abs_file, new_size);
                                drop(new_file);
                                return;
                            }

                            let Ok((old_file, old_file_size)) = mmap_open_sync(&prev_file) else {
                                new_file!(shared_self, rel_file, abs_file, new_size);
                                drop(new_file);
                                return;
                            };

                            if new_size != old_file_size {
                                debug!(file = %prev_file.display(), "Not deduplicating: File sizes different, so it must have changed.");
                                changed_file!(shared_self, rel_file, abs_file, new_size);
                                drop(new_file);
                                drop(old_file);
                                return;
                            }

                            let new_file_buf = new_file.as_slice();
                            let old_file_buf = old_file.as_slice();

                            if !memx::memeq(new_file_buf, old_file_buf) {
                                debug!(file = %prev_file.display(), "Not deduplicating: File content differs.");
                                changed_file!(shared_self, rel_file, abs_file, new_size);
                                drop(new_file);
                                drop(old_file);
                                return;
                            }

                            drop(new_file);
                            drop(old_file);

                            // File is the same, so dedup it.
                            shared_self.cfg.dedup_tmp(&abs_file, &latest_snapshot);
                        });
                    }
                });
                debug!(chain = %shared_self.cfg.chain, "Background DeDup : Finished");
            });
        });

        (tx, dedup_processor)
    }
}

/// Use a consistent name for a download archive to simplify processing.
const DOWNLOAD_FILE_NAME: &str = "latest-mithril.tar.zst";

/// Check if wew pre-downloaded the file to ~/Downloads and of so, jut use that.
/// Saves time in testing.
async fn check_pre_downloaded(dest: &Path, url: &str) -> bool {
    let file_name_parts: Vec<&str> = url.split('/').collect();
    if let Some(file_name) = file_name_parts.last() {
        if let Some(mut dl_path) = dirs::download_dir() {
            dl_path.push(file_name);
            if dl_path.is_file() {
                let mut dest_file = dest.to_path_buf();
                dest_file.push(DOWNLOAD_FILE_NAME);
                if dest_file.exists() {
                    if let Err(error) = tokio::fs::remove_file(&dest_file).await {
                        error!(error=%error, "Destination File Exists");
                    }
                }

                if let Err(error) = hard_link(dl_path, dest_file).await {
                    error!(error=%error, "Trying to hard link pre downloaded file.");
                } else {
                    return true;
                }
            }
        }
    }

    debug!("Failed to find pre-downloaded file. Downloading now.");

    false
}

/// Download a file using `aria2` tool,s with maximum number of simultaneous connections.
async fn aria2_download(dest: &Path, url: &str) -> MithrilResult<()> {
    // Use pre-downloaded file if it exists
    if check_pre_downloaded(dest, url).await {
        return Ok(());
    }

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
        bail!("aria2c stderr channel was not readable.");
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

/// Open a file using mmap for performance.
async fn mmap_open(path: &Path) -> MithrilResult<AsyncMmapFile> {
    match AsyncMmapFile::open_with_options(path, AsyncOptions::new().read(true).populate()).await {
        Ok(file) => Ok(file),
        Err(error) => {
            error!(error=%error, file=%path.to_string_lossy(), "Failed to open file");
            Err(error.into())
        },
    }
}

/// Open a file using mmap for performance.
fn mmap_open_sync(path: &Path) -> MithrilResult<(fmmap::MmapFile, u64)> {
    match fmmap::MmapFile::open_with_options(path, fmmap::Options::new().read(true).populate()) {
        Ok(file) => {
            let len = file.len() as u64;
            Ok((file, len))
        },
        Err(error) => {
            error!(error=%error, file=%path.to_string_lossy(), "Failed to open file");
            Err(error.into())
        },
    }
}

#[async_trait]
impl SnapshotDownloader for MithrilTurboDownloader {
    async fn download_unpack(
        &self, location: &str, target_dir: &Path, _compression_algorithm: CompressionAlgorithm,
        _download_id: &str, _snapshot_size: u64,
    ) -> MithrilResult<()> {
        self.create_directories(target_dir).await?;

        let dst_archive = self.dl(location, target_dir).await?;

        // Decompress and extract and de-dupe each file in the archive.
        stats::mithril_extract_started(self.inner.cfg.chain);

        debug!(
            "Unpacking and extracting '{}' to '{}'.",
            dst_archive.to_string_lossy(),
            target_dir.to_string_lossy()
        );

        let file = mmap_open(&dst_archive).await?;

        let reader = file.reader(0)?;
        let mut archive = Archive::new(ZstdDecoder::new(reader));

        debug!("Extracting files from compressed archive.");

        let mut entries = archive.entries()?;
        let tmp_dir = self.inner.cfg.tmp_path();

        // Start a dedup thread which does all this work below inside a parallel runner.
        let (dedup_queue, dedup_join_handle) = self.start_dedup();

        // let latest_snapshot = self.inner.cfg.latest_snapshot_id().await;
        while let Some(file) = entries.next().await {
            let mut file = file?;

            // Unpack the raw file first.
            file.unpack_in(tmp_dir.clone()).await?;

            // Supply the runner with files to dedup from a queue.
            // When finished, close the queue.
            // And wait for the de-duper to finish (which it does when it detects the queue is
            // closed.) This will let us dedup in parallel with file extraction adn
            // untar. The de-duper is CPU heavy, so use sync IO for the de-dupe process.
            let Ok(file_path) = file.path() else {
                error!("Failed to get the path for de-duping...");
                continue;
            };
            if dedup_queue.send(file_path.to_path_buf()).is_err() {
                error!("Failed to send file to de-duper...");
                continue;
            };
        }

        // Close the de-dup queue, and wait for the de-duper to finish its pending work.
        drop(dedup_queue);
        if dedup_join_handle.await.is_err() {
            error!("Failed to wait for de-duper to finish...");
        }

        let tot_files = self.inner.tot_files.load(Ordering::SeqCst);
        let chg_files = self.inner.chg_files.load(Ordering::SeqCst);
        let new_files = self.inner.new_files.load(Ordering::SeqCst);

        stats::mithril_extract_finished(
            self.inner.cfg.chain,
            Some(self.inner.ext_size.load(Ordering::SeqCst)),
            self.inner.ddup_size.load(Ordering::SeqCst),
            tot_files - (chg_files + new_files),
            chg_files,
            new_files,
        );

        debug!("Download and Unpack finished='{location}' to '{target_dir:?}'.");

        Ok(())
    }

    async fn probe(&self, location: &str) -> MithrilResult<()> {
        debug!("HEAD Snapshot location='{location}'.");

        let request_builder = self.inner.http_client.head(location);
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
