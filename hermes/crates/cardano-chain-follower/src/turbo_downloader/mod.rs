//! Serializable Parallel Download Processor
//!
//! Provides the capability to quickly download a large file using parallel connections,
//! but still process the data sequentially, without requiring the entire file to be
//! downloaded at once.
//!
//! NOTE: This uses synchronous threading and HTTP Gets because Async proved to be highly
//! variable in its performance.

use std::{
    io::Read,
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use dashmap::DashMap;
use http::{
    header::{ACCEPT_RANGES, CONTENT_LENGTH, RANGE},
    StatusCode,
};
use tracing::{debug, error};

use crate::utils::u64_from_saturating;

/// A Simple DNS Balancing Resolver
struct BalancingResolver {
    /// The actual resolver
    resolver: hickory_resolver::Resolver,
    /// A Cache of the Sockets we already resolved for a URL.
    cache: moka::sync::Cache<String, Arc<Vec<SocketAddr>>>,
}

/// We only have one resolver.
static RESOLVER: OnceLock<BalancingResolver> = OnceLock::new();

impl BalancingResolver {
    /// Initialize the resolver, only does something once, but safe to call multiple
    /// times.
    fn init(_cfg: &DlConfig) -> Result<()> {
        // Can ONLY init the Resolver once, just return if we try and do it multiple times.
        if RESOLVER.get().is_none() {
            // Construct a new Resolver with default configuration options
            let resolver = hickory_resolver::Resolver::new(
                hickory_resolver::config::ResolverConfig::default(),
                hickory_resolver::config::ResolverOpts::default(),
            )?;

            let cache = moka::sync::Cache::builder()
                // We should nto be caching lots of different URL's
                .max_capacity(10)
                // Time to live (TTL): 60 minutes
                .time_to_live(Duration::from_secs(60 * 60))
                // Time to idle (TTI):  5 minutes
                .time_to_idle(Duration::from_secs(5 * 60))
                // Create the cache.
                .build();

            // We don't really care if this is already set.
            let _unused = RESOLVER.set(BalancingResolver { resolver, cache });
        }
        Ok(())
    }

    /// Resolve the given URL with the configured resolver.
    fn resolve(&self, url: &str, worker: usize) -> std::io::Result<Vec<std::net::SocketAddr>> {
        // debug!("Resolving: {url} for {worker}");
        let addresses = if let Some(addresses) = self.cache.get(url) {
            addresses
        } else {
            let Some((host, port_str)) = url.split_once(':') else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Could not parse URL",
                ));
            };

            let port: u16 = port_str.parse().map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Could not parse port number",
                )
            })?;

            let mut all_addresses: Vec<std::net::SocketAddr> = Vec::new();
            for addr in self.resolver.lookup_ip(host.to_string())?.iter() {
                all_addresses.push(std::net::SocketAddr::new(addr, port));
            }

            let addresses = Arc::new(all_addresses);
            self.cache.insert(url.to_string(), addresses.clone());
            addresses
        };
        let worker_addresses = worker % addresses.len();
        // Safe because we bound the index with the length of `addresses`.
        #[allow(clippy::indexing_slicing)]
        Ok(vec![addresses[worker_addresses]])
    }
}

// Timeout if connection can not be made in 10 seconds.
// const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

// Timeout if no data received for 5 seconds.
// const DATA_READ_TIMEOUT: Duration = Duration::from_secs(5);

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

    /// Resolve DNS addresses using Hickory Resolver
    fn resolve(url: &str, worker: usize) -> std::io::Result<Vec<std::net::SocketAddr>> {
        let Some(resolver) = RESOLVER.get() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Resolver not initialized.",
            ));
        };

        resolver.resolve(url, worker)
    }

    /// Builds a `UReq` Agent.  
    ///
    /// Because we need multiple clients to prevent all traffic being forced onto a single
    /// connection when HTTP2 is used, the client can NOT be supplied by the user.
    /// Instead we create a new one here based on their configuration.
    pub(crate) fn make_http_agent(&self, worker: usize) -> ureq::Agent {
        let mut agent = ureq::AgentBuilder::new();

        if let Some(timeout) = self.connection_timeout {
            agent = agent.timeout_connect(timeout);
        }

        if let Some(timeout) = self.data_read_timeout {
            agent = agent.timeout_read(timeout);
        }

        let agent = agent.resolver(move |url: &str| Self::resolve(url, worker));

        agent.build()
    }
}

impl Default for DlConfig {
    fn default() -> Self {
        DlConfig {
            workers: 32,
            chunk_size: 8 * 1024 * 1024,
            queue_ahead: 3,
            connection_timeout: None,
            data_read_timeout: None,
        }
    }
}

/// An Individual Downloaded block of data.
/// Wrapped in an ARC so its cheap to clone and pass between threads.
type DlBlock = Arc<Vec<u8>>;

/// Downloaded Chunk (or error if it fails).
#[derive(Clone)]
struct DlChunk {
    /// Index of the worker that fetched the chunk.
    worker: usize,
    /// Index of the chunk in the file.
    chunk_num: usize,
    /// The data from the chunk. (None == failed)
    chunk: Option<DlBlock>,
}

/// Download Chunk Work Order.
/// This is simply the number of the chunk next to fetch.
/// When finished, the queue is just closed.
type DlWorkOrder = usize;

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
    reorder_queue: DashMap<usize, DlChunk>,
    /// A queue for each worker to send them new work orders.
    work_queue: DashMap<usize, crossbeam_channel::Sender<DlWorkOrder>>,
    /// New Chunk Queue - Just says we added a new chunk to the reorder queue.
    new_chunk_queue_tx: crossbeam_channel::Sender<Option<()>>,
    /// New Chunk Queue - Just says we added a new chunk to the reorder queue.
    new_chunk_queue_rx: crossbeam_channel::Receiver<Option<()>>,
    /// Statistic tracking number of bytes downloaded per worker.
    bytes_downloaded: Vec<AtomicU64>,
    /// Left Over Bytes (from the reader)
    left_over_bytes: Mutex<Option<(Arc<Vec<u8>>, usize)>>,
    /// Next Expected Chunk
    next_expected_chunk: AtomicUsize,
    /// Next Chunk to Request
    next_requested_chunk: AtomicUsize,
}

impl Drop for ParallelDownloadProcessorInner {
    /// Cleanup the channel and workers.
    fn drop(&mut self) {
        debug!("Drop ParallelDownloadProcessorInner");
        self.reorder_queue.clear();
        self.reorder_queue.shrink_to_fit();
        self.work_queue.clear();
        self.work_queue.shrink_to_fit();
    }
}

impl ParallelDownloadProcessorInner {
    /// Get how many bytes were downloaded, total.
    pub(crate) fn total_bytes(&self) -> u64 {
        self.bytes_downloaded
            .iter()
            .map(|x| x.load(Ordering::SeqCst))
            .sum::<u64>()
    }

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
    fn get_range(&self, agent: &ureq::Agent, chunk: usize) -> anyhow::Result<Arc<Vec<u8>>> {
        let range_start = self.chunk_start(chunk);
        let range_end_inclusive = self.chunk_end(chunk);
        let range_header = format!("bytes={range_start}-{range_end_inclusive}");
        let get_range_response = agent
            .get(&self.url)
            .set(RANGE.as_str(), &range_header)
            .call()
            .context("GET ranged request failed")?;
        let addr = get_range_response.remote_addr();
        debug!("Chunk {chunk} from {addr:?}");
        if get_range_response.status() != StatusCode::PARTIAL_CONTENT {
            bail!(
                "Response to range request has an unexpected status code (expected {}, found {})",
                StatusCode::PARTIAL_CONTENT,
                get_range_response.status()
            )
        }

        let range_size = range_end_inclusive - range_start + 1;
        let mut bytes: Vec<u8> = Vec::with_capacity(range_size);

        let bytes_read = get_range_response
            .into_reader()
            .take(u64_from_saturating(range_size))
            .read_to_end(&mut bytes)?;

        if bytes_read != range_size {
            bail!("Expected {range_size} bytes in response, but only read {bytes_read}")
        }

        Ok(Arc::new(bytes))
    }

    /// Queue Chunk to processor.
    ///
    /// Reorders chunks and sends to the consumer.
    fn reorder_queue(&self, chunk: DlChunk) -> anyhow::Result<()> {
        self.reorder_queue.insert(chunk.chunk_num, chunk);
        self.new_chunk_queue_tx.send(Some(()))?;
        Ok(())
    }
}

/// Parallel Download Processor.
///
/// Uses multiple connection to speed up downloads, but returns data sequentially
/// so it can be processed without needing to store the whole file in memory or disk.
#[derive(Clone)]
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
        let file_size = get_content_length_async(url).await?;

        // Get the minimum number of workers we need, just in case the chunk size is bigger than
        // the requested workers can process.
        cfg.workers = file_size.div_ceil(cfg.chunk_size).min(cfg.workers);

        let last_chunk = file_size.div_ceil(cfg.chunk_size);

        // Initialize the download statistics
        let mut bytes_downloaded = Vec::with_capacity(cfg.workers);
        for _ in 0..cfg.workers {
            bytes_downloaded.push(AtomicU64::new(0));
        }

        let new_chunk_queue = crossbeam_channel::unbounded();

        let processor = ParallelDownloadProcessor(Arc::new(ParallelDownloadProcessorInner {
            url: String::from(url),
            cfg: cfg.clone(),
            file_size,
            last_chunk,
            reorder_queue: DashMap::with_capacity((cfg.workers * cfg.queue_ahead) + 1),
            work_queue: DashMap::with_capacity(cfg.workers + 1),
            new_chunk_queue_rx: new_chunk_queue.1,
            new_chunk_queue_tx: new_chunk_queue.0,
            bytes_downloaded,
            left_over_bytes: Mutex::new(None),
            next_expected_chunk: AtomicUsize::new(0),
            next_requested_chunk: AtomicUsize::new(0),
        }));

        processor.start_workers()?;

        Ok(processor)
    }

    /// Starts the worker tasks, they will not start doing any work until `download` is
    /// called, which happens immediately after they are started.
    fn start_workers(&self) -> anyhow::Result<()> {
        for worker in 0..self.0.cfg.workers {
            // The channel is unbounded, because work distribution is controlled to be at most
            // `work_queue` deep per worker. And we don't want anything unexpected to
            // cause the processor to block.
            let (work_queue_tx, work_queue_rx) = crossbeam_channel::unbounded::<DlWorkOrder>();
            let params = self.0.clone();
            thread::spawn(move || {
                Self::worker(&params, worker, &work_queue_rx);
            });

            let _unused = self.0.work_queue.insert(worker, work_queue_tx);
        }

        self.download()
    }

    /// The worker task - It is running in parallel and downloads chunks of the file as
    /// requested.
    fn worker(
        params: &Arc<ParallelDownloadProcessorInner>, worker_id: usize,
        work_queue: &crossbeam_channel::Receiver<DlWorkOrder>,
    ) {
        debug!("Worker {worker_id} started");

        // Each worker has its own http_client, so there is no cross worker pathology
        // Each worker should be expected to make multiple requests to the same host.
        // Resolver should never fail to initialize.  However, if it does, we can;t start the
        // worker.
        if let Err(error) = BalancingResolver::init(&params.cfg) {
            error!("Failed to initialize DNS resolver for worker {worker_id}: {error:?}");
            return;
        }
        let http_agent = params.cfg.make_http_agent(worker_id);

        while let Ok(next_chunk) = work_queue.recv() {
            let mut retries = 0;
            let mut block;
            debug!("Worker {worker_id} DL chunk {next_chunk}");
            loop {
                block = match params.get_range(&http_agent, next_chunk) {
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

            if let Some(ref block) = block {
                if let Some(dl_stat) = params.bytes_downloaded.get(worker_id) {
                    let this_bytes_downloaded = u64_from_saturating(block.len());
                    let _last_bytes_downloaded = dl_stat
                        .fetch_add(this_bytes_downloaded, std::sync::atomic::Ordering::SeqCst);
                    // debug!("Worker {worker_id} DL chunk {next_chunk}:
                    // {last_bytes_downloaded} + {this_bytes_downloaded} = {}",
                    // last_bytes_downloaded+this_bytes_downloaded);
                } else {
                    error!("Failed to get bytes downloaded for worker {worker_id}");
                }
            }

            if let Err(error) = params.reorder_queue(DlChunk {
                worker: worker_id,
                chunk_num: next_chunk,
                chunk: block,
            }) {
                error!("Error sending chunk: {:?}, error: {:?}", next_chunk, error);
                break;
            };
            debug!("Worker {worker_id} DL chunk queued {next_chunk}");
        }
        debug!("Worker {worker_id} ended");
    }

    /// Send a work order to a worker.
    fn send_work_order(&self, this_worker: usize, order: DlWorkOrder) -> Result<usize> {
        let params = self.0.clone();
        let next_worker = (this_worker + 1) % params.cfg.workers;
        if let Some(worker_queue) = params.work_queue.get(&this_worker) {
            let queue = worker_queue.value();
            queue.send(order)?;
        } else {
            bail!("Expected a work queue for worker: {:?}", this_worker);
        }
        Ok(next_worker)
    }

    /// Starts Downloading the file using parallel connections.
    ///
    /// Should only be called once on self.
    fn download(&self) -> anyhow::Result<()> {
        let params = self.0.clone();
        // Pre fill the work queue with orders.
        let max_pre_orders = params.cfg.queue_ahead * params.cfg.workers;
        let pre_orders = max_pre_orders.min(params.last_chunk);

        let mut this_worker: usize = 0;

        // Fill up the pre-orders into the workers queues.
        for pre_order in 0..pre_orders {
            this_worker = self.send_work_order(this_worker, pre_order)?;
        }

        params
            .next_requested_chunk
            .store(pre_orders, Ordering::SeqCst);

        Ok(())
    }

    /// Get current size of data we downloaded.
    pub(crate) fn dl_size(&self) -> u64 {
        self.0.total_bytes()
    }
}

impl std::io::Read for ParallelDownloadProcessor {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // There should only ever be one reader, the purpose of this mutex is to give us
        // mutability it should never actually block.
        let mut left_over_buffer = self
            .0
            .left_over_bytes
            .lock()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{e:?}")))?;

        let (left_over_bytes, offset) =
            if let Some((left_over_bytes, offset)) = left_over_buffer.take() {
                (left_over_bytes, offset)
            } else {
                // Get the next chunk and inc the one we would want next.
                let next_chunk = self.0.next_expected_chunk.fetch_add(1, Ordering::SeqCst);

                // Wait here until we actually have the next chunk in the reorder queue.
                while !self.0.reorder_queue.contains_key(&next_chunk) {
                    if let Err(error) = self.0.new_chunk_queue_rx.recv() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Next Chunk Queue Error: {error:?}"),
                        ));
                    }
                }

                let Some((_, chunk)) = self.0.reorder_queue.remove(&next_chunk) else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Expected Chunk {next_chunk} Didn't get any"),
                    ));
                };

                if chunk.chunk_num != next_chunk {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Expected Chunk {next_chunk} Got {}", chunk.chunk_num),
                    ));
                }
                let Some(ref block) = chunk.chunk else {
                    return Ok(0); // EOF
                };

                // Got a chunk so lets queue more work from the worker that gave us this block.
                // Because we are pre-incrementing here, its possible for this to be > maximum
                // chunks and thats OK.
                let next_work_order = self.0.next_requested_chunk.fetch_add(1, Ordering::SeqCst);

                // Send more work to the worker that just finished a work order.
                if next_work_order < self.0.last_chunk {
                    if let Err(error) = self.send_work_order(chunk.worker, next_work_order) {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Failed to send work order to {} : {error:?}", chunk.worker),
                        ));
                    }
                }

                (block.to_owned(), 0)
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
            *left_over_buffer = Some((left_over_bytes, offset + bytes_to_copy));
        }

        Ok(bytes_to_copy)
    }
}

/// Send a HEAD request to obtain the length of the file we want to download (necessary
/// for calculating the offsets of the chunks)
///
/// This exists because the `Probe` call made by Mithril is Async, and this makes
/// interfacing to that easier.
async fn get_content_length_async(url: &str) -> anyhow::Result<usize> {
    let url = url.to_owned();
    match tokio::task::spawn_blocking(move || get_content_length(&url)).await {
        Ok(result) => result,
        Err(error) => {
            error!("get_content_length failed");
            Err(anyhow::anyhow!("get_content_length failed: {}", error))
        },
    }
}

/// Send a HEAD request to obtain the length of the file we want to download (necessary
/// for calculating the offsets of the chunks)
fn get_content_length(url: &str) -> anyhow::Result<usize> {
    let response = ureq::head(url).call()?;

    if response.status() != StatusCode::OK {
        bail!(
            "HEAD request did not return a successful response: {}",
            response.status_text()
        );
    }

    if let Some(accept_ranges) = response.header(ACCEPT_RANGES.as_str()) {
        if accept_ranges != "bytes" {
            bail!(
                "Server doesn't support HTTP range byte requests (Accept-Ranges = {})",
                accept_ranges
            );
        }
    } else {
        bail!("Server doesn't support HTTP range requests (missing ACCEPT_RANGES header)");
    };

    let content_length = if let Some(content_length) = response.header(CONTENT_LENGTH.as_str()) {
        let content_length: usize = content_length
            .parse()
            .context("Content-Length was not a valid unsigned integer")?;
        content_length
    } else {
        bail!("HEAD response did not contain a Content-Length header");
    };

    Ok(content_length)
}
