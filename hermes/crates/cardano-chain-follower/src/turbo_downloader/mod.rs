//! Serializable Parallel Download Processor
//!
//! Provides the capability to quickly download a large file using parallel connections, but still
//! process the data sequentially, without requiring the entire file to be downloaded at once.

use anyhow::{bail, Context, Result};

use crossbeam_skiplist::SkipMap;
use reqwest::header::{ACCEPT_RANGES, CONTENT_LENGTH, RANGE};
use reqwest::StatusCode;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver, Sender, UnboundedReceiver, UnboundedSender};
use tokio::sync::{Mutex, RwLock};
use tokio_util::bytes::Bytes;
use tracing::error;

// Timeout if connection can not be made in 10 seconds.
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

// Timeout if no data received for 5 seconds.
const DATA_READ_TIMEOUT: Duration = Duration::from_secs(5);

// Minimum rational size of a chunk in bytes.
const MIN_CHUNK_SIZE: usize = 1024 * 4; // 4 KB

/// Downloaded Chunk (or error if it fails).
#[derive(Clone)]
struct DlChunk {
    worker: usize,
    chunk_num: usize,
    chunk: Option<Bytes>,
}

/// Download Chunk Work Order.
/// This is simply the number of the chunk next to fetch.
/// When finished, the queue is just closed.
type DlWorkOrder = usize;

/// Parallel Download Processor Inner struct.
///
/// Note: Maximum Potential Working set in memory will ==  `dl_chunk` * ((`workers` * `queue_ahead`) + 1)
struct ParallelDownloadProcessorInner {
    // URL to download from.
    url: String,
    /// HTTP Client to use for requests.
    http_client: reqwest::Client,
    // Number of workers to use.
    workers: usize,
    // Size of the file we expect to download.
    file_size: usize,
    // Chunk size to download in parallel (in bytes).
    dl_chunk_size: usize,
    // The last chunk we can request
    last_chunk: usize,
    // How many chunks are queued ahead per worker (max).
    queue_ahead: usize,
    // Skip map used to reorder incoming chunks back into sequential order.
    reorder_queue: SkipMap<usize, DlChunk>,
    // The next required/expected chunk to send in order.
    next_chunk: RwLock<usize>,
    // A queue for each worker to send them new work orders.
    work_queue: SkipMap<usize, UnboundedSender<DlWorkOrder>>,
    // The Data Stream Queue used to send data to a Reader.
    stream_queue: Mutex<(Option<(Bytes, usize)>, Receiver<Option<Bytes>>)>,
    // Realtime Download Statistics, because we really do need them.
    stats: SkipMap<String, Option<()>>, // Just a placeholder for now.
}

impl ParallelDownloadProcessorInner {
    /// Get start offset of a chunk.
    fn chunk_start(&self, chunk: usize) -> usize {
        return self.dl_chunk_size * chunk;
    }

    /// Get inclusive end offset of a chunk.
    fn chunk_end(&self, chunk: usize) -> usize {
        let start = self.chunk_start(chunk);
        let end = if start + self.dl_chunk_size >= self.file_size {
            self.file_size - 1
        } else {
            start + self.dl_chunk_size - 1
        };
        return end;
    }

    /// Sends a GET request to download a chunk of the file at the specified range
    async fn get_range(&self, chunk: usize) -> anyhow::Result<tokio_util::bytes::Bytes> {
        let range_start = self.chunk_start(chunk);
        let range_end_inclusive = self.chunk_end(chunk);
        let range_header = format!("bytes={range_start}-{range_end_inclusive}");
        let get_range_response = self
            .http_client
            .get(&self.url)
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

    /// Check if we need to send `DlChunk` to the consumer, or queue it for re ordering.
    async fn check_to_send(&self, chunk: DlChunk) -> Option<DlChunk> {
        let next = self.next_chunk.read().await;
        if chunk.chunk_num == *next {
            return Some(chunk);
        }
        self.reorder_queue.insert(chunk.chunk_num, chunk);
        None
    }

    /// Queue Chunk to processor.
    ///
    /// Reorders chunks and sends to the consumer.
    async fn reorder_queue(
        &self, chunk: DlChunk, result_queue_tx: &UnboundedSender<DlChunk>,
    ) -> anyhow::Result<()> {
        if let Some(chunk) = self.check_to_send(chunk).await {
            // Send first consecutive chunk without needing to insert into reorder queue first.
            result_queue_tx.send(chunk)?;

            // If we should be sending, then get a write lock, so we can do it without race conditions.
            let mut next = self.next_chunk.write().await;
            let mut actual_next = *next + 1;

            // Send any blocks that are consecutive from the reorder queue.
            while self.reorder_queue.contains_key(&actual_next) {
                let Some(entry) = self.reorder_queue.pop_front() else {
                    bail!("Expected to find a chunk in the reorder queue, but did not")
                };
                let chunk = entry.value();
                result_queue_tx.send(chunk.clone())?;

                actual_next += 1;
            }
            *next = actual_next;
        }

        Ok(())
    }
}

/// Parallel Download Processor.
///
/// Uses multiple connection to speed up downloads, but returns data sequentially
/// so it can be processed without needing to store the whole file in memory or disk.
#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct ParallelDownloadProcessor(Arc<ParallelDownloadProcessorInner>);

impl ParallelDownloadProcessor {
    /// Creates a new instance of the Parallel Download Processor.
    ///
    /// Can Fail IF there is no HTTP client provided or the URL does not support getting the content length.
    pub(crate) async fn new(
        url: &str, workers: usize, dl_chunk_size: usize, queue_ahead: usize,
        client: Option<reqwest::Client>,
    ) -> anyhow::Result<Self> {
        if dl_chunk_size < MIN_CHUNK_SIZE {
            bail!(
                "Download chunk size must be at least {} bytes",
                MIN_CHUNK_SIZE
            );
        }
        let http_client = match client {
            Some(c) => c,
            None => reqwest::ClientBuilder::new()
                .connect_timeout(CONNECTION_TIMEOUT)
                .read_timeout(DATA_READ_TIMEOUT)
                .build()?,
        };
        let file_size = get_content_length(&http_client, url).await?;
        let last_chunk = file_size.div_ceil(dl_chunk_size);
        let (stream_queue_tx, stream_queue_rx) = mpsc::channel::<Option<Bytes>>(2);
        let processor = ParallelDownloadProcessor(Arc::new(ParallelDownloadProcessorInner {
            url: String::from(url),
            http_client,
            workers,
            file_size,
            dl_chunk_size,
            last_chunk,
            queue_ahead,
            reorder_queue: SkipMap::new(),
            next_chunk: RwLock::new(0),
            work_queue: SkipMap::new(),
            stream_queue: Mutex::new((None, stream_queue_rx)),
            stats: SkipMap::new(),
        }));

        processor.start_workers(stream_queue_tx);

        Ok(processor)
    }

    /// Starts the worker tasks, they will not start doing any work until `download` is called, which happens immediately after they are started.
    fn start_workers(&self, stream_queue_tx: Sender<Option<Bytes>>) {
        let (result_queue_tx, result_queue_rx) = mpsc::unbounded_channel::<DlChunk>();
        for worker in 0..self.0.workers {
            // The channel is unbounded, because work distribution is controlled to be at most `work_queue` deep per worker.
            // And we don't want anything unexpected to cause the processor to block.
            let (work_queue_tx, work_queue_rx) = mpsc::unbounded_channel::<DlWorkOrder>();
            let tx_queue = result_queue_tx.clone();
            let params = self.0.clone();
            tokio::spawn(async move {
                Self::worker(params, worker, work_queue_rx, tx_queue).await;
            });

            let _unused = self.0.work_queue.insert(worker, work_queue_tx);
        }

        let params = self.0.clone();
        tokio::spawn(async move {
            if let Err(error) = Self::download(params, result_queue_rx, stream_queue_tx).await {
                error!("Failed to download with: {:?}", error);
            }
        });
    }

    /// The worker task - It is running in parallel and downloads chunks of the file as requested.
    async fn worker(
        params: Arc<ParallelDownloadProcessorInner>, worker_id: usize,
        mut work_queue: UnboundedReceiver<DlWorkOrder>, result_queue: UnboundedSender<DlChunk>,
    ) {
        while let Some(next_chunk) = work_queue.recv().await {
            let mut retries = 0;
            let mut block;
            loop {
                block = match params.get_range(next_chunk).await {
                    Ok(block) => Some(block),
                    Err(error) => {
                        error!("Error getting chunk: {:?}, error: {:?}", next_chunk, error);
                        None
                    },
                };

                // Quickly retry on error, in case its transient.
                if block.is_some() || retries > 3 {
                    break;
                }
                retries += 1;
            }

            if let Err(error) = params
                .reorder_queue(
                    DlChunk {
                        worker: worker_id,
                        chunk_num: next_chunk,
                        chunk: block,
                    },
                    &result_queue,
                )
                .await
            {
                error!("Error sending chunk: {:?}, error: {:?}", next_chunk, error);
                break;
            };
        }
    }

    /// Send a work order to a worker.
    fn send_work_order(
        params: &Arc<ParallelDownloadProcessorInner>, this_worker: usize, order: DlWorkOrder,
    ) -> Result<usize> {
        let next_worker = (this_worker + 1) % params.workers;
        if let Some(worker_queue) = params.work_queue.get(&this_worker) {
            let queue = worker_queue.value();
            queue.send(order)?;
        } else {
            bail!("Expected a work queue for worker: {:?}", this_worker);
        }
        Ok(next_worker)
    }

    /// Downloads the file using parallel connections.
    ///
    /// Can only be called once on Self.
    async fn download(
        params: Arc<ParallelDownloadProcessorInner>, mut rx_queue: UnboundedReceiver<DlChunk>,
        stream_queue_tx: Sender<Option<Bytes>>,
    ) -> anyhow::Result<()> {
        // Pre fill the work queue with orders.
        let max_pre_orders = params.queue_ahead * params.workers;
        let pre_orders = max_pre_orders.min(params.last_chunk);

        let mut this_worker: usize = 0;
        let mut next_expected_chunk: usize = 0;

        // Fill up the pre-orders into the workers queues.
        for pre_order in 0..pre_orders {
            this_worker = Self::send_work_order(&params, this_worker, pre_order)?;
        }

        let mut next_work_order = pre_orders;

        // Wait for blocks to come back from the workers.
        // Issue new orders until we either send them all, OR we get an error.
        // Terminate once we have received all the blocks.
        while let Some(chunk) = rx_queue.recv().await {
            // Check the chunk is the one we expected.
            if chunk.chunk_num != next_expected_chunk {
                bail!(
                    "Received unexpected chunk, expected {}, got {}",
                    next_expected_chunk,
                    chunk.chunk_num
                );
            }

            if chunk.chunk_num >= params.last_chunk {
                break;
            }
            next_expected_chunk += 1;

            // Send more work to the worker that just finished a work order.
            if next_work_order < params.last_chunk {
                let _unused = Self::send_work_order(&params, chunk.worker, next_work_order)?;
                next_work_order += 1;
            }

            // Send the chunk to the consumer...
            // This only has a very small buffer, so DL rate will be limited to consumption rate.
            stream_queue_tx.send(chunk.chunk).await?;
        }

        Ok(())
    }
}

impl std::io::Read for ParallelDownloadProcessor {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut rx_queue = self.0.stream_queue.blocking_lock();

        let (left_over_bytes, offset) = if let Some((left_over_bytes, offset)) = rx_queue.0.take() {
            (left_over_bytes, offset)
        } else {
            let Some(block) = rx_queue.1.blocking_recv().flatten() else {
                return Ok(0); // EOF
            };
            (block, 0)
        };

        // Send whats leftover or new.
        let bytes_left = left_over_bytes.len() - offset;
        let bytes_to_copy = bytes_left.min(buf.len());
        let Some(sub_buf) = left_over_bytes.get(offset..offset + bytes_to_copy) else {
            error!("Slicing Sub Buffer failed");
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Slicing Sub Buffer failed",
            ));
        };
        if let Err(error) = memx::memcpy(buf, sub_buf) {
            error!(error=?error, "memx::memcpy failed");
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "memx::memcpy failed",
            ));
        }

        // Save whats leftover back inside the mutex, if there is anything.
        if offset + bytes_to_copy != left_over_bytes.len() {
            rx_queue.0 = Some((left_over_bytes, offset + bytes_to_copy));
        }

        Ok(bytes_to_copy)
    }
}

/// Send a HEAD request to obtain the length of the file we want to download (necessary for
/// calculating the offsets of the chunks)
async fn get_content_length(http_client: &reqwest::Client, url: &str) -> anyhow::Result<usize> {
    let head_response = http_client
        .head(url)
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
    let content_length: usize = content_length
        .parse()
        .context("Content-Length was not a valid unsigned integer")?;

    Ok(content_length)
}
