use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
    time::Instant,
};

use anyhow::bail;
use anyhow::{anyhow, Context};
use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use lz4_flex::frame::FrameDecoder;
use reqwest::header::{ACCEPT_RANGES, CONTENT_LENGTH, RANGE};
use reqwest::StatusCode;
use std::sync::atomic::AtomicU64;
use tar::Archive;
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};
use tracing::{debug, error, info};
// A unique reference to a slice of memory
use bytes::BytesMut;

use crate::{
    engine::{decode_loop, download_loop, init_download_loop},
    options::TurboDownloaderOptions,
    progress::{InternalProgress, UnpackedFileInfo},
    tsutils::TimePair,
    utils::{bytes_to_human, resolve_url},
    wrapper::{DataChunk, MpscReaderFromReceiver},
    TurboDownloaderProgress,
};

/// Created from [TurboDownloaderOptions]
pub struct TurboDownloader {
    url: Arc<String>,
    progress_context: Arc<Mutex<InternalProgress>>,
    options: TurboDownloaderOptions,
    download_started: AtomicBool,
    target_path: PathBuf,
    next_chunk: AtomicU64,
    total_chunks: AtomicU64,
    file_size: AtomicU64,
    client: reqwest::Client,
    decoder_handle: Arc<Option<JoinHandle<anyhow::Result<()>>>>,
    decoder_tx_chunk: Option<Sender<NextChunk>>,
}

struct NextChunk {
    chunk_no: u64,
    chunk_start: u64,
    chunk_size: u64,
    data: Option<bytes::Bytes>,
}

impl TurboDownloader {
    pub fn new(
        url: &str, target_path: PathBuf, turbo_downloader_options: TurboDownloaderOptions,
    ) -> Self {
        Self {
            url: url.to_string().into(),
            progress_context: Arc::new(Mutex::new(InternalProgress::default())),
            download_started: AtomicBool::new(false),
            target_path,
            options: turbo_downloader_options,
            next_chunk: AtomicU64::new(0),
            file_size: AtomicU64::new(0),
            total_chunks: AtomicU64::new(0),
            client: reqwest::Client::new(),
            decoder_handle: Arc::new(None),
            decoder_tx_chunk: None,
        }
    }

    fn get_next_chunk(&self) -> Option<NextChunk> {
        // Get the next chunk, and increment it.
        let next = self.next_chunk.fetch_add(1, Ordering::Relaxed);
        // Get the absolute max chunk we can have.
        let max_chunk = self.total_chunks.load(Ordering::Relaxed);
        if next < max_chunk {
            let chunk_size = self.options.dl_chunk_size;
            let chunk_start = next * chunk_size;

            Some(NextChunk {
                chunk_no: next,
                chunk_start,
                chunk_size,
                data: None,
            })
        } else {
            None
        }
    }

    /// Process inputs and try to start download
    pub async fn start(self: &mut TurboDownloader) -> anyhow::Result<()> {
        // We can ONLY start a download once. Fail if someone tries to do it twice.
        if self.download_started.swap(true, Ordering::Relaxed) {
            return Err(anyhow::anyhow!("Download already started"));
        }

        // Get the file size and save it.
        self.set_content_length().await?;

        // Create the decode/extract task.
        self.start_decoder().await;

        Ok(())
    }

    /// Start the decoder task.
    async fn start_decoder(self: &mut TurboDownloader) {
        let (tx, mut rx) = mpsc::channel::<NextChunk>(1);

        self.decoder_tx_chunk = Some(tx);
        let y = self.decoder_task(rx);
        let x = Some(tokio::spawn(y));
    }

    async fn decoder_task(
        self: &mut TurboDownloader, _rx: Receiver<NextChunk>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// Get the length of the file from the server.
    async fn set_content_length(self: &mut TurboDownloader) -> anyhow::Result<()> {
        let head_response = self
            .client
            .head(self.url.as_str())
            .send()
            .await
            .context("HEAD request failed")?;

        head_response
            .error_for_status_ref()
            .context("HEAD request returned non-success status code")?;

        let Some(accept_ranges) = head_response.headers().get(ACCEPT_RANGES) else {
            bail!("Server doesn't support HTTP range requests (missing ACCEPT_RANGES header)");
        };

        let accept_ranges = String::from_utf8_lossy(accept_ranges.as_bytes());
        if accept_ranges != "bytes" {
            bail!("Server doesn't support HTTP range requests (Accept-Ranges = {accept_ranges})");
        }
        let Some(content_length) = head_response.headers().get(CONTENT_LENGTH) else {
            bail!("HEAD response did not contain a Content-Length header");
        };
        let content_length = content_length
            .to_str()
            .context("Content-Length header contained invalid UTF8")?;
        let content_length: u64 = content_length
            .parse()
            .context("Content-Length was not a valid 64-bit unsigned integer")?;
        if content_length == 0 {
            bail!("The file you are trying to download has zero length");
        }

        self.file_size.store(content_length, Ordering::Relaxed);

        Ok(())
    }

    // Sends a GET request to download a chunk of the file at the specified range
    async fn get_range(
        self: &mut TurboDownloader, chunk: &NextChunk,
    ) -> anyhow::Result<bytes::Bytes> {
        let range_start = chunk.chunk_start;
        let range_end = chunk.chunk_size + range_start - 1; // inclusive
        let range_header = format!("bytes={range_start}-{range_end}");

        let get_range_response = self
            .client
            .get(self.url.as_str())
            .header(RANGE, range_header)
            .send()
            .await
            .context("GET request failed")?;
        get_range_response
            .error_for_status_ref()
            .context("GET request returned non-success status code")?;
        if get_range_response.status() != StatusCode::PARTIAL_CONTENT {
            bail!(
                "Response to range request has an unexpected status code (expected {}, found {})",
                StatusCode::PARTIAL_CONTENT,
                get_range_response.status()
            )
        }
        let body = get_range_response
            .bytes()
            .await
            .context("error while streaming body")?;

        Ok(body)
    }
}

fn tar_unpack(
    dst: &Path, tar: &mut Archive<MpscReaderFromReceiver>, options: TurboDownloaderOptions,
    pc: Arc<Mutex<InternalProgress>>,
) -> std::io::Result<()> {
    if dst.symlink_metadata().is_err() {
        fs::create_dir_all(dst)?
    }

    // Canonicalizing the dst directory will prepend the path with '\\?\'
    // on windows which will allow windows APIs to treat the path as an
    // extended-length path with a 32,767 character limit. Otherwise all
    // unpacked paths over 260 characters will fail on creation with a
    // NotFound exception.
    let dst = &dst.canonicalize().unwrap_or(dst.to_path_buf());

    // Delay any directory entries until the end (they will be created if needed by
    // descendants), to ensure that directory permissions do not interfer with descendant
    // extraction.
    let mut directories = Vec::new();
    for entry in tar.entries()? {
        let mut file = entry?;
        debug!(
            "entry: {:?}, path {}",
            file.header().entry_type(),
            file.path()?.display()
        );
        if options.ignore_symlinks && file.header().entry_type() == tar::EntryType::Symlink {
            continue;
        }
        if options.ignore_symlinks && file.header().entry_type() == tar::EntryType::Link {
            continue;
        }
        let file_header_name = file.path()?.display().to_string();
        let file_header_size = file.header().size().unwrap_or(0);
        {
            let mut pc = pc.lock().unwrap();
            let file_no = pc.unpacked_files + 1;
            pc.last_unpacked_files.push_back(UnpackedFileInfo {
                file_no,
                file_name: file_header_name,
                file_size: file_header_size,
                finished: false,
            });
            if pc.last_unpacked_files.len() > 10 {
                pc.last_unpacked_files.pop_front();
            }
        }

        if file.header().entry_type() == tar::EntryType::Directory {
            directories.push(file);
        } else {
            file.unpack_in(dst)?;
        }
        {
            let mut pc = pc.lock().unwrap();
            if let Some(last_unp_file) = pc.last_unpacked_files.back_mut() {
                last_unp_file.finished = true;
            }
            pc.unpacked_files += 1;
        }
    }

    for mut dir in directories {
        dir.unpack_in(dst)?;
    }
    Ok(())
}
