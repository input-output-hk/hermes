//! Turbo Downloads for Mithril Snapshots.

use std::{
    cmp,
    ffi::OsStr,
    io::{BufReader, Read},
    path::{Path, PathBuf},
    // process::Stdio,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, OnceLock,
    },
};

use anyhow::{anyhow, bail};
// use async_compression::tokio::bufread::ZstdDecoder;
use async_trait::async_trait;
use crossbeam_skiplist::SkipSet;
use fmmap::{
    // tokio::{AsyncMmapFile, AsyncMmapFileExt, AsyncOptions},
    MmapFileExt,
};
use memx::memcmp;
use mithril_client::{
    common::CompressionAlgorithm, snapshot_downloader::SnapshotDownloader, MithrilResult,
};
use tar::{Archive, EntryType};
use tokio::{
    // fs::{create_dir_all, symlink},
    fs::create_dir_all,
    // process::Command,
    // sync::mpsc::{self, UnboundedSender},
    task::spawn_blocking,
    // task::{spawn_blocking, JoinHandle},
};
// use tokio_stream::StreamExt;
// use tokio_util::codec::{FramedRead, LinesCodec};
use tracing::{debug, error};
use zstd::Decoder;

use crate::{
    mithril_snapshot_config::MithrilSnapshotConfig,
    mithril_snapshot_data::latest_mithril_snapshot_data,
    stats::{self},
    turbo_downloader::ParallelDownloadProcessor,
    utils::usize_from_saturating,
};

/// A snapshot downloader that accelerates Download using `aria2`.
pub struct Inner {
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

    /// The download processor for the current file download.
    dl_handler: std::sync::OnceLock<ParallelDownloadProcessor>,
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

/// This macro is what happens every time we decide the file can't be deduplicated.
macro_rules! new_file {
    ($self:ident, $rel_file:ident, $abs_file:ident, $new_size:ident) => {
        $self.new_files.fetch_add(1, Ordering::SeqCst);
        $self.ddup_size.fetch_add($new_size, Ordering::SeqCst);
        if $abs_file.extension() == Some(OsStr::new("chunk")) {
            $self.new_chunks.insert($abs_file);
        }
    };
}

impl Inner {
    /// Synchronous Download and Dedup archive.
    ///
    /// Stream Downloads and Decompresses files, and deduplicates them as they are
    /// extracted from the embedded tar archive.
    ///
    /// Per Entry:
    ///   If the file is NOT to be deduplicated, OR A previous file with the same name and
    /// size does not     exist, then just extract it where its supposed to go.
    ///
    /// To Dedup, the original file is mam-mapped.
    /// The new file is extracted to an in-memory buffer.
    /// If they compare the same, the original file is `HardLinked` to the new file name.
    /// Otherwise the new file buffer is saved to disk with the new file name.
    fn dl_and_dedup(&self, location: &str, target_dir: &Path) -> MithrilResult<()> {
        let mut archive = self.create_archive_extractor()?;

        // Iterate the files in the archive.
        let entries = match archive.entries() {
            Ok(entries) => entries,
            Err(error) => bail!("Failed to get entries from the archive: {error}"),
        };

        for entry in entries {
            let mut entry = match entry {
                Ok(entry) => entry,
                Err(error) => bail!("Failed to get an entry from the archive: {error}"),
            };
            let rel_file = entry.path()?.to_path_buf();
            let entry_size = entry.size();

            debug!(chain = %self.cfg.chain, "Background DeDup : Extracting {}:{} loc {location} target {}", rel_file.to_string_lossy(), entry_size, target_dir.to_string_lossy());

            // Check if we need to extract this path or not.
            if !self.check_for_extract(&rel_file, entry.header().entry_type()) {
                continue;
            }

            let tmp_dir = self.cfg.tmp_path();
            let latest_snapshot = latest_mithril_snapshot_data(self.cfg.chain);

            let mut abs_file = tmp_dir.clone();
            abs_file.push(rel_file.clone());

            let mut prev_file = latest_snapshot.id().path_if_exists();
            if let Some(prev_file) = &mut prev_file {
                prev_file.push(rel_file.clone());
            }

            debug!(chain = %self.cfg.chain, "Background DeDup : tmp_dir {} abs_file {} prev_file {prev_file:?}", tmp_dir.to_string_lossy(), abs_file.to_string_lossy() );

            self.ext_size.fetch_add(entry_size, Ordering::SeqCst);

            // Try and deduplicate the file if we can, otherwise just extract it.
            if let Ok((prev_mmap, _)) = Self::can_deduplicate(&rel_file, entry_size, &prev_file) {
                let expected_file_size = usize_from_saturating(entry_size);
                let mut buf: Vec<u8> = Vec::with_capacity(expected_file_size);
                if entry.read_to_end(&mut buf)? != expected_file_size {
                    bail!(
                        "Failed to read file {} of size {} got {}",
                        rel_file.display(),
                        entry_size,
                        buf.len()
                    );
                }
                // Got the full file and its the expected size.  Is it different?
                if memcmp(prev_mmap.as_slice(), buf.as_slice()) == cmp::Ordering::Equal {
                    // Same so lets Hardlink it, and throw away the temp buffer.

                    // Make sure our big mmap get dropped.
                    drop(prev_mmap);

                    // File is the same, so dedup it.
                    if self.cfg.dedup_tmp(&abs_file, &latest_snapshot).is_ok() {
                        self.ddup_size.fetch_add(entry_size, Ordering::SeqCst);
                        changed_file!(self, rel_file, abs_file, entry_size);
                        drop(buf);
                        continue;
                    }
                }

                if let Err(error) = std::fs::write(&abs_file, buf) {
                    error!(chain = %self.cfg.chain, "Failed to write file {} got {}", abs_file.display(), error);
                    bail!("Failed to write file {} got {}", abs_file.display(), error);
                }
            } else {
                // No dedup, just extract it into the tmp directory as-is.
                entry.unpack_in(tmp_dir)?;
                debug!(chain = %self.cfg.chain, "Extracted file {:?}:{}", rel_file, entry_size);
            }
            new_file!(self, rel_file, abs_file, entry_size);
        }

        let Some(dl_handler) = self.dl_handler.get() else {
            bail!("Failed to get the Parallel Download processor!");
        };

        debug!(chain = %self.cfg.chain, "Download {} bytes", dl_handler.dl_size());

        stats::mithril_dl_finished(self.cfg.chain, Some(dl_handler.dl_size()));

        Ok(())
    }

    /// Create a TAR archive extractor from the downloading file and a zstd decompressor.
    fn create_archive_extractor(
        &self,
    ) -> MithrilResult<Archive<Decoder<'static, BufReader<BufReader<ParallelDownloadProcessor>>>>>
    {
        let Some(dl_handler) = self.dl_handler.get() else {
            bail!("Failed to get the Parallel Download processor!");
        };
        let buf_reader = BufReader::new(dl_handler.clone());
        let decoder = match zstd::Decoder::new(buf_reader) {
            Ok(decoder) => decoder,
            Err(error) => bail!("Failed to create ZSTD decoder: {error}"),
        };
        Ok(tar::Archive::new(decoder))
    }

    /// Check if we are supposed to extract this file from the archive or not.
    fn check_for_extract(&self, path: &Path, etype: EntryType) -> bool {
        if path.is_absolute() {
            error!(chain = %self.cfg.chain, "Background DeDup : Cannot extract an absolute path:  {:?}", path);
            return false;
        }

        if etype.is_dir() {
            // We don't do anything with just a path, so skip it.
            return false;
        }

        if !etype.is_file() {
            error!(chain  = %self.cfg.chain, "Background DeDup : Cannot extract a non-file: {:?}:{:?}", path, etype);
            return false;
        }

        true
    }

    /// Check if a given path from the archive is able to be deduplicated.
    fn can_deduplicate(
        rel_file: &Path, file_size: u64, prev_file: &Option<PathBuf>,
    ) -> MithrilResult<(fmmap::MmapFile, u64)> {
        // Can't dedup if the current file is not de-dupable (must be immutable)
        if rel_file.starts_with("immutable") {
            // Can't dedup if we don't have a previous file to dedup against.
            if let Some(prev_file) = prev_file {
                if let Some(current_size) = get_file_size_sync(prev_file) {
                    // If the current file is not exactly the same as the previous file size, we
                    // can't dedup.
                    if file_size == current_size {
                        if let Ok(pref_file_loaded) = mmap_open_sync(prev_file) {
                            if pref_file_loaded.1 == file_size {
                                return Ok(pref_file_loaded);
                            }
                        }
                    }
                }
            }
        }
        bail!("Can not deduplicate.");
    }
}

/// A snapshot downloader that accelerates Download using `aria2`.
pub struct MithrilTurboDownloader {
    /// inner arc wrapped configuration
    inner: Arc<Inner>,
}

impl MithrilTurboDownloader {
    /// Constructs a new `HttpSnapshotDownloader`.
    pub fn new(cfg: MithrilSnapshotConfig) -> Self {
        // Test if the HTTP Client can properly be created.
        let dl_config = cfg.dl_config.clone().unwrap_or_default();

        let cfg = cfg.with_dl_config(dl_config);

        Self {
            inner: Arc::new(Inner {
                cfg,
                new_chunks: Arc::new(SkipSet::new()),
                new_files: AtomicU64::new(0),
                chg_files: AtomicU64::new(0),
                tot_files: AtomicU64::new(0),
                ext_size: AtomicU64::new(0),
                ddup_size: AtomicU64::new(0),
                dl_handler: OnceLock::new(),
            }),
        }
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

    // Actually download the file to a temp directory first.
    // async fn dl(&self, location: &str, target_dir: &Path) -> MithrilResult<PathBuf> {
    // self.create_directories(target_dir).await?;
    //
    // debug!("Download and Unpack started='{location}' to '{target_dir:?}'.");
    //
    // stats::mithril_dl_started(self.inner.cfg.chain);
    //
    // First Download the Archive using Aria2 to the `dl` directory.
    // TODO(SJ): Using `aria2` as a convenience, need to change to a rust native
    // multi-connection download crate, which needs to be written.
    // if let Err(error) = aria2_download(&self.inner.cfg.dl_path(), location).await {
    // Record failed download stats
    // stats::mithril_dl_finished(self.inner.cfg.chain, None);
    // return Err(error);
    // }
    //
    // let mut dst_archive = self.inner.cfg.dl_path();
    // dst_archive.push(DOWNLOAD_FILE_NAME);
    //
    // Record successful download stats
    // stats::mithril_dl_finished(
    // self.inner.cfg.chain,
    // get_file_size(dst_archive.clone()).await,
    // );
    //
    // Ok(dst_archive)
    // }
    //
    // Start the Dedup Workers.
    // Allows us to overlap De-duping and Unpacking.
    // fn start_dedup(&self) -> (UnboundedSender<PathBuf>, JoinHandle<()>) {
    // Maximum number of files pending to dedup simultaneously.
    // let (tx, mut rx) = mpsc::unbounded_channel::<PathBuf>();
    //
    // let shared_self = self.inner.clone();
    //
    // let dedup_processor = tokio::spawn(async move {
    // debug!(chain = %shared_self.cfg.chain, "Background DeDup : Started");
    //
    // tokio::task::spawn_blocking(move || {
    // rayon::scope(|s| {
    // while let Some(dedup_this) = rx.blocking_recv() {
    // let shared_self = shared_self.clone();
    // s.spawn(move |_| {
    // let rel_file = dedup_this.clone();
    // let tmp_dir = shared_self.cfg.tmp_path();
    // let latest_snapshot =
    // latest_mithril_snapshot_data(shared_self.cfg.chain);
    //
    // let mut abs_file = tmp_dir.clone();
    // abs_file.push(rel_file.clone());
    //
    // let mut prev_file = latest_snapshot.id().path();
    // prev_file.push(rel_file.clone());
    //
    // debug!(
    //    "Comparing File: {} with {} ",
    //    abs_file.to_string_lossy(),
    //    prev_file.to_string_lossy()
    // );
    //
    // We ONLY de-dupe files.
    // if !abs_file.is_file() {
    // debug!("No Dedup, Not a File '{}'.", rel_file.to_string_lossy());
    // return;
    // }
    //
    // shared_self.tot_files.fetch_add(1, Ordering::SeqCst);
    //
    // let Ok((new_file, new_size)) = mmap_open_sync(&abs_file) else {
    // return;
    // };
    //
    // shared_self.ext_size.fetch_add(new_size, Ordering::SeqCst);
    //
    // Assume that any files not in the immutable directory have changed, and don't check them
    // if !rel_file.starts_with(Path::new("immutable")) {
    // debug!(file = %rel_file.display(), "Not deduplicating: Not immutable");
    // new_file!(shared_self, rel_file, abs_file, new_size);
    // drop(new_file);
    // return;
    // }
    //
    // If the previous file doesn't exist, or isn't a file, then the file is new.
    // if !prev_file.is_file() {
    // debug!(file = %prev_file.display(), "Not deduplicating: Previous File does not exist or
    // is not a file."); new_file!(shared_self, rel_file, abs_file, new_size);
    // drop(new_file);
    // return;
    // }
    //
    // let Ok((old_file, old_file_size)) = mmap_open_sync(&prev_file) else {
    // new_file!(shared_self, rel_file, abs_file, new_size);
    // drop(new_file);
    // return;
    // };
    //
    // if new_size != old_file_size {
    // debug!(file = %prev_file.display(), "Not deduplicating: File sizes different, so it
    // must have changed."); changed_file!(shared_self, rel_file, abs_file, new_size);
    // drop(new_file);
    // drop(old_file);
    // return;
    // }
    //
    // let new_file_buf = new_file.as_slice();
    // let old_file_buf = old_file.as_slice();
    //
    // if !memx::memeq(new_file_buf, old_file_buf) {
    // debug!(file = %prev_file.display(), "Not deduplicating: File content differs.");
    // changed_file!(shared_self, rel_file, abs_file, new_size);
    // drop(new_file);
    // drop(old_file);
    // return;
    // }
    //
    // drop(new_file);
    // drop(old_file);
    //
    // File is the same, so dedup it.
    // let _UNUSED = shared_self.cfg.dedup_tmp(&abs_file, &latest_snapshot);
    // });
    // }
    // });
    // debug!(chain = %shared_self.cfg.chain, "Background DeDup : Finished");
    // });
    // });
    //
    // (tx, dedup_processor)
    // }

    /// Parallel Download, Extract and Dedup the Mithril Archive.
    async fn dl_and_dedup(&self, location: &str, target_dir: &Path) -> MithrilResult<()> {
        // Get a copy of the inner data to use in the sync download task.
        let inner = self.inner.clone();
        let location = location.to_owned();
        let target_dir = target_dir.to_owned();

        // This is fully synchronous IO, so do it on a sync thread.
        let result = spawn_blocking(move || inner.dl_and_dedup(&location, &target_dir)).await;

        if let Ok(result) = result {
            return result;
        }

        stats::mithril_dl_finished(self.inner.cfg.chain, None);
        bail!("Download and Dedup task failed");
    }
}

#[allow(dead_code, clippy::missing_docs_in_private_items)]
const _UNUSED_CHECK_PRE_DOWNLOADED: &str = r#"

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

                if let Err(error) = symlink(dl_path, dest_file).await {
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
"#;

#[allow(dead_code, clippy::missing_docs_in_private_items)]
const _UNUSED_ARIA2_DOWNLOAD: &str = r#"

/// Download a file using `aria2` tools with maximum number of simultaneous connections.
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
"#;

#[allow(dead_code, clippy::missing_docs_in_private_items)]
const _UNUSED_GET_FILE_SIZE: &str = r"
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
";

/// Get the size of a particular file.  None = failed to get size (doesn't matter why).
fn get_file_size_sync(file: &Path) -> Option<u64> {
    let Ok(metadata) = file.metadata() else {
        return None;
    };
    Some(metadata.len())
}

#[allow(dead_code, clippy::missing_docs_in_private_items)]
const _UNUSED_MMAP_OPEN_ASYNC: &str = r#"
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
"#;

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

        // DL Start stats set after DL actually started inside the probe call.
        self.dl_and_dedup(location, target_dir).await?;

        #[allow(dead_code, clippy::missing_docs_in_private_items)]
        let _unused: &str = r#"
        let dst_archive = self.dl(location, target_dir).await?;

        debug!(
            "Unpacking and extracting '{}' to '{}'.",
            dst_archive.to_string_lossy(),
            target_dir.to_string_lossy()
        );

        let file = mmap_open(&dst_archive).await?;

        let reader = file.reader(0)?;
        let mut archive = tokio_tar::Archive::new(ZstdDecoder::new(reader));

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
        "#;

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
        debug!("Probe Snapshot location='{location}'.");

        let dl_config = self.inner.cfg.dl_config.clone().unwrap_or_default();
        let dl_processor = ParallelDownloadProcessor::new(location, dl_config).await?;

        // Decompress and extract and de-dupe each file in the archive.
        stats::mithril_extract_started(self.inner.cfg.chain);

        // We also immediately start downloading now.
        stats::mithril_dl_started(self.inner.cfg.chain);

        // Save the DownloadProcessor in the inner struct for use to process the downloaded data.
        if let Err(_error) = self.inner.dl_handler.set(dl_processor) {
            bail!("Failed to set the inner dl_handler. Must already be set?");
        }

        // let request_builder = self.inner.http_client.head(location);
        // let response = request_builder.send().await.with_context(|| {
        // format!("Cannot perform a HEAD for snapshot at location='{location}'")
        // })?;
        //
        // let status = response.status();
        //
        // debug!("Probe for '{location}' completed: {status}");
        //
        // match response.status() {
        // reqwest::StatusCode::OK => Ok(()),
        // reqwest::StatusCode::NOT_FOUND => {
        // Err(anyhow!("Snapshot location='{location} not found"))
        // },
        // status_code => Err(anyhow!("Unhandled error {status_code}")),
        // }

        Ok(())
    }
}
