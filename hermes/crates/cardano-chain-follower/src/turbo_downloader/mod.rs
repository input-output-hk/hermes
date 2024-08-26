//! Serializable Parallel Download Processor
//!
//! Provides the capability to quickly download a large file using parallel connections,
//! but still process the data sequentially, without requiring the entire file to be
//! downloaded at once.

use std::{sync::Arc, time::Duration};

use anyhow::{bail, Context, Result};
use crossbeam_skiplist::SkipMap;
use reqwest::{
    header::{ACCEPT_RANGES, CONTENT_LENGTH, RANGE},
    StatusCode,
};
use tokio::sync::{
    mpsc::{self, Receiver, Sender, UnboundedReceiver, UnboundedSender},
    Mutex, RwLock,
};
use tokio_util::bytes::Bytes;
use tracing::{debug, error};

/// Timeout if connection can not be made in 10 seconds.
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

/// Timeout if no data received for 5 seconds.
const DATA_READ_TIMEOUT: Duration = Duration::from_secs(5);

/// Minimum rational size of a chunk in bytes.
const MIN_CHUNK_SIZE: usize = 1024 * 4; // 4 KB

/// Parallel Downloader Tuning parameters
#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct DlConfig {
    /// Maximum number of parallel connections to use.
    pub workers: usize,
    /// Size of a chunk in bytes (except the last).
    pub chunk_size: usize,
    /// Maximum number of chunks queued ahead to workers.
    pub queue_ahead: usize,
    /// Timeout for each connection.
    pub connection_timeout: Option<Duration>,
    /// Timeout for each data read.
    pub data_read_timeout: Option<Duration>,
    /// HTTP1 Forced - If both HTTP1 and HTTP2 are forced, then its auto-detected, same if
    /// neither is forced.
    pub http1_forced: bool,
    /// HTTP2 Forced - If both HTTP1 and HTTP2 are forced, then its auto-detected, same if
    /// neither is forced.
    pub http2_forced: bool,
    /// HTTP2 uses an adaptive window size.
    pub http2_adaptive_window: bool,
    /// HTTP2 Keep Alive while Idle
    pub http2_keep_alive_while_idle: bool,
    /// HTTP2 keep alive interval
    pub http2_keep_alive_interval: Option<Duration>,
    /// HTTP2 Maximum Frame Size
    pub http2_max_frame_size: Option<u32>,
    /// HTTP2 initial connection window size in bytes.
    pub http2_initial_connection_window_size: Option<u32>,
    /// HTTP2 initial stream window size in bytes.
    pub http2_initial_stream_window_size: Option<u32>,
}

impl DlConfig {
    /// Create a new `DlConfig`
    pub fn new() -> Self {
        DlConfig::default()
    }

    /// Change the number of workers
    pub fn with_workers(mut self, workers: usize) -> Self {
        self.workers = workers;
        self
    }

    /// Change the chunk size
    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size;
        self
    }

    /// Change the number of chunks queued ahead to workers
    pub fn with_queue_ahead(mut self, queue_ahead: usize) -> Self {
        self.queue_ahead = queue_ahead;
        self
    }

    /// Change the connection timeout
    pub fn with_connection_timeout(mut self, connection_timeout: Duration) -> Self {
        self.connection_timeout = Some(connection_timeout);
        self
    }

    /// Change the data read timeout
    pub fn with_data_read_timeout(mut self, data_read_timeout: Duration) -> Self {
        self.data_read_timeout = Some(data_read_timeout);
        self
    }

    /// Is HTTP1 to be forced?
    pub fn with_http1(mut self) -> Self {
        self.http1_forced = true;
        self
    }

    /// Is HTTP2 to be forced?
    pub fn with_http2(mut self) -> Self {
        self.http2_forced = true;
        self
    }

    /// HTTP2 use adaptive window size?
    pub fn with_adaptive_window(mut self) -> Self {
        self.http2_adaptive_window = true;
        self
    }

    /// HTTP2 keep alive while idle?
    pub fn with_keepalive_while_idle(mut self) -> Self {
        self.http2_keep_alive_while_idle = true;
        self
    }

    /// HTTP2 keep alive interval?
    pub fn with_keepalive_interval(mut self, interval: Duration) -> Self {
        self.http2_keep_alive_interval = Some(interval);
        self
    }

    /// HTTP2 set maximum frame size
    pub fn with_max_frame_size(mut self, max_frame_size: u32) -> Self {
        self.http2_max_frame_size = Some(max_frame_size);
        self
    }

    /// HTTP2 set initial connection window size
    pub fn with_initial_connection_window(mut self, initial_connection_window: u32) -> Self {
        self.http2_initial_connection_window_size = Some(initial_connection_window);
        self
    }

    /// HTTP2 set initial stream window size
    pub fn with_initial_stream_window(mut self, initial_stream_window_size: u32) -> Self {
        self.http2_initial_stream_window_size = Some(initial_stream_window_size);
        self
    }

    /// Builds a Reqwest client.  
    ///
    /// Because we need multiple clients to prevent all traffic being forced onto a single
    /// connection when HTTP2 is used, the client can NOT be supplied by the user.
    /// Instead we create a new one here based on their configuration.
    pub(crate) fn make_http_conn(&self) -> Result<reqwest::Client> {
        let mut client_builder = reqwest::ClientBuilder::new();

        if self.http1_forced && !self.http2_forced {
            client_builder = client_builder.http1_only();
        };

        if self.http2_forced && !self.http1_forced {
            client_builder = client_builder.http2_prior_knowledge();
        };

        if let Some(timeout) = self.connection_timeout {
            client_builder = client_builder.connect_timeout(timeout);
        }

        if let Some(timeout) = self.data_read_timeout {
            client_builder = client_builder.read_timeout(timeout);
        }

        if self.http2_adaptive_window {
            client_builder = client_builder.http2_adaptive_window(true);
        }

        if self.http2_keep_alive_while_idle {
            client_builder = client_builder.http2_keep_alive_while_idle(true);
        }

        client_builder = client_builder.http2_keep_alive_interval(self.http2_keep_alive_interval);
        client_builder = client_builder.http2_max_frame_size(self.http2_max_frame_size);
        client_builder = client_builder
            .http2_initial_connection_window_size(self.http2_initial_connection_window_size);

        Ok(client_builder.build()?)
    }
}

impl Default for DlConfig {
    fn default() -> Self {
        DlConfig {
            workers: 16,
            chunk_size: 2 * 1024 * 1024,
            queue_ahead: 3,
            connection_timeout: None,
            data_read_timeout: None,
            http1_forced: false,
            http2_forced: true,
            http2_adaptive_window: true,
            http2_keep_alive_while_idle: false,
            http2_keep_alive_interval: None,
            http2_max_frame_size: None,
            http2_initial_connection_window_size: None,
            http2_initial_stream_window_size: None,
        }
    }
}

/// Downloaded Chunk (or error if it fails).
#[derive(Clone)]
struct DlChunk {
    /// Index of the worker that fetched the chunk.
    worker: usize,
    /// Index of the chunk in the file.
    chunk_num: usize,
    /// The data from the chunk. (None == failed)
    #[allow(dead_code)]
    chunk: Option<Bytes>,
}

/// Download Chunk Work Order.
/// This is simply the number of the chunk next to fetch.
/// When finished, the queue is just closed.
type DlWorkOrder = usize;

/// A Stream Queue and residual block storage.
type StreamQueue = (Option<(Bytes, usize)>, Receiver<Option<Bytes>>);

/// Parallel Download Processor Inner struct.
///
/// Note: Maximum Potential Working set in memory will ==  `dl_chunk` * ((`workers` *
/// `queue_ahead`) + 1)
struct ParallelDownloadProcessorInner {
    /// URL to download from.
    url: String,
    /// Configuration
    cfg: DlConfig,
    /// Size of the file we expect to download.
    file_size: usize,
    /// The last chunk we can request
    last_chunk: usize,
    /// Skip map used to reorder incoming chunks back into sequential order.
    reorder_queue: SkipMap<usize, DlChunk>,
    /// The next required/expected chunk to send in order.
    next_chunk: RwLock<usize>,
    /// A queue for each worker to send them new work orders.
    work_queue: SkipMap<usize, UnboundedSender<DlWorkOrder>>,
    /// The Data Stream Queue used to send data to a Reader.
    stream_queue: Mutex<StreamQueue>,
    /// Realtime Download Statistics, because we really do need them.
    #[allow(dead_code)]
    stats: SkipMap<String, Option<()>>, // Just a placeholder for now.
}

impl ParallelDownloadProcessorInner {
    /// Get start offset of a chunk.
    fn chunk_start(&self, chunk: usize) -> usize {
        self.cfg.chunk_size * chunk
    }

    /// Get inclusive end offset of a chunk.
    fn chunk_end(&self, chunk: usize) -> usize {
        let start = self.chunk_start(chunk);
        if start + self.cfg.chunk_size >= self.file_size {
            self.file_size - 1
        } else {
            start + self.cfg.chunk_size - 1
        }
    }

    /// Sends a GET request to download a chunk of the file at the specified range
    async fn get_range(
        &self, http_client: &reqwest::Client, chunk: usize,
    ) -> anyhow::Result<tokio_util::bytes::Bytes> {
        let range_start = self.chunk_start(chunk);
        let range_end_inclusive = self.chunk_end(chunk);
        let range_header = format!("bytes={range_start}-{range_end_inclusive}");
        let get_range_response = http_client
            .get(&self.url)
            .header(RANGE, range_header)
            .send()
            .await
            .context("GET request failed")?;
        let addr = get_range_response.remote_addr();
        debug!("Chunk {chunk} from {addr:?}");
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

            // If we should be sending, then get a write lock, so we can do it without race
            // conditions.
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
    /// Can Fail IF there is no HTTP client provided or the URL does not support getting
    /// the content length.
    pub(crate) async fn new(url: &str, mut cfg: DlConfig) -> anyhow::Result<Self> {
        if cfg.chunk_size < MIN_CHUNK_SIZE {
            bail!(
                "Download chunk size must be at least {} bytes",
                MIN_CHUNK_SIZE
            );
        }
        let http_client = reqwest::ClientBuilder::new()
            .connect_timeout(CONNECTION_TIMEOUT)
            .read_timeout(DATA_READ_TIMEOUT)
            .build()?;
        let file_size = get_content_length(&http_client, url).await?;

        // Get the minimum number of workers we need, just in case the chunk size is bigger than
        // the requested workers can process.
        cfg.workers = file_size.div_ceil(cfg.chunk_size).min(cfg.workers);

        let last_chunk = file_size.div_ceil(cfg.chunk_size);
        let (stream_queue_tx, stream_queue_rx) = mpsc::channel::<Option<Bytes>>(2);
        let processor = ParallelDownloadProcessor(Arc::new(ParallelDownloadProcessorInner {
            url: String::from(url),
            cfg: cfg.clone(),
            file_size,
            last_chunk,
            reorder_queue: SkipMap::new(),
            next_chunk: RwLock::new(0),
            work_queue: SkipMap::new(),
            stream_queue: Mutex::new((None, stream_queue_rx)),
            stats: SkipMap::new(),
        }));

        processor.start_workers(stream_queue_tx);

        Ok(processor)
    }

    /// Starts the worker tasks, they will not start doing any work until `download` is
    /// called, which happens immediately after they are started.
    fn start_workers(&self, stream_queue_tx: Sender<Option<Bytes>>) {
        let (result_queue_tx, result_queue_rx) = mpsc::unbounded_channel::<DlChunk>();
        for worker in 0..self.0.cfg.workers {
            // The channel is unbounded, because work distribution is controlled to be at most
            // `work_queue` deep per worker. And we don't want anything unexpected to
            // cause the processor to block.
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

    /// The worker task - It is running in parallel and downloads chunks of the file as
    /// requested.
    async fn worker(
        params: Arc<ParallelDownloadProcessorInner>, worker_id: usize,
        mut work_queue: UnboundedReceiver<DlWorkOrder>, result_queue: UnboundedSender<DlChunk>,
    ) {
        debug!("Worker {worker_id} started");
        // Each worker has its own http_client, so there is no cross worker pathology
        // Each worker should be expected to make multiple requests to the same host.
        let http_client = match params.cfg.make_http_conn() {
            Ok(client) => client,
            Err(error) => {
                error!("Failed to create http1 client: {error}");
                return;
            },
        };

        while let Some(next_chunk) = work_queue.recv().await {
            let mut retries = 0;
            let mut block;
            debug!("Worker {worker_id} DL chunk {next_chunk}");
            loop {
                block = match params.get_range(&http_client, next_chunk).await {
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
            debug!("Worker {worker_id} DL chunk done {next_chunk}: {retries}");

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
            debug!("Worker {worker_id} DL chunk queued {next_chunk}");
        }
        debug!("Worker {worker_id} ended");
    }

    /// Send a work order to a worker.
    fn send_work_order(
        params: &Arc<ParallelDownloadProcessorInner>, this_worker: usize, order: DlWorkOrder,
    ) -> Result<usize> {
        let next_worker = (this_worker + 1) % params.cfg.workers;
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
        let max_pre_orders = params.cfg.queue_ahead * params.cfg.workers;
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
            //#[allow(clippy::no_effect_underscore_binding)]
            // let _unused = &stream_queue_tx;
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

/// Send a HEAD request to obtain the length of the file we want to download (necessary
/// for calculating the offsets of the chunks)
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
