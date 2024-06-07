use crate::tsutils::TimePair;
use chrono::Utc;
use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};
use std::time;
use std::time::Instant;
use tracing::trace;

#[derive(Debug, Clone, Serialize)]
pub struct ProgressHistoryEntry {
    #[serde(skip_serializing)]
    time: time::Instant,
    bytes: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct ProgressHistory {
    progress_entries: Vec<ProgressHistoryEntry>,
    max_entries: usize,
    keep_time: time::Duration,
}
impl Default for ProgressHistory {
    fn default() -> Self {
        Self {
            progress_entries: Vec::new(),
            max_entries: 10,
            keep_time: time::Duration::from_secs(1),
        }
    }
}
impl ProgressHistory {
    pub fn new() -> ProgressHistory {
        ProgressHistory {
            progress_entries: vec![],
            max_entries: 50,
            keep_time: time::Duration::from_secs(10),
        }
    }

    pub fn get_speed(&self) -> usize {
        //warn!("First enty from {}", self.progress_entries.get(0).map(|entry| std::time::Instant::now() - entry.time).unwrap_or());
        let current_time = time::Instant::now();
        let mut last_time = current_time
            .checked_sub(self.keep_time)
            .unwrap_or(current_time);
        let mut total: usize = 0;
        //let now = std::time::Instant::now();
        for entry in self.progress_entries.iter().rev() {
            if current_time - entry.time > self.keep_time {
                break;
            }
            total += entry.bytes;
            last_time = entry.time;
        }
        let elapsed_secs = time::Instant::now() - last_time;
        //let elapsed_secs = time::Instant::now - last_time (chrono::Utc::now() - last_time).num_milliseconds() as f64 / 1000.0;
        trace!("Progress entries count {}", self.progress_entries.len());
        trace!("Last entry: {:?}", elapsed_secs);

        (total as f64 / elapsed_secs.as_secs_f64()).round() as usize
    }

    pub fn add_bytes(&mut self, bytes: usize) {
        let current_time = time::Instant::now();
        if let Some(last_entry) = self.progress_entries.last_mut() {
            if current_time - last_entry.time < (self.keep_time / (self.max_entries as u32)) {
                last_entry.bytes += bytes;
                return;
            }
        }
        self.progress_entries.push(ProgressHistoryEntry {
            time: current_time,
            bytes,
        });

        //this should be removed max one time
        assert!(self.progress_entries.len() <= self.max_entries + 1);
        while self.progress_entries.len() > self.max_entries {
            //warn!("ProgressHistory: max_entries reached");
            self.progress_entries.remove(0);
        }

        //remove old entries
        while let Some(first) = self.progress_entries.first() {
            if current_time - first.time > self.keep_time {
                self.progress_entries.remove(0);
            } else {
                break;
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadChunkProgress {
    pub downloaded: usize,
    pub to_download: usize,
    pub unpacked: usize,
    pub to_unpack: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnpackedFileInfo {
    pub file_no: usize,
    pub file_name: String,
    pub file_size: u64,
    pub finished: bool,
}

#[derive(Debug, Clone)]
pub struct InternalProgress {
    pub start_time: TimePair,
    pub chunk_size: usize,
    pub unfinished_chunks: Vec<usize>,
    pub current_chunks: BTreeMap<usize, DownloadChunkProgress>,
    //pub unpack_chunks: BTreeMap<usize, UnpackChunkProgress>,
    pub total_chunks: usize,
    pub total_downloaded: usize,
    pub total_download_size: Option<usize>,
    pub chunk_downloaded: Vec<usize>,
    pub total_unpacked: usize,
    pub total_unpack_size: Option<usize>,
    pub stop_requested: bool,
    pub paused: bool,
    pub progress_buckets_download: ProgressHistory,
    pub progress_buckets_unpack: ProgressHistory,
    pub finish_time: Option<TimePair>,
    pub error_time: Option<time::Instant>,
    pub error_message_download: Option<String>,
    pub error_message_unpack: Option<String>,
    pub error_message: Option<String>,
    pub download_url: Option<String>,
    pub download_threads: usize,
    pub server_chunk_support: bool,
    pub unpacked_files: usize,
    pub last_unpacked_files: VecDeque<UnpackedFileInfo>,
}

impl Default for InternalProgress {
    fn default() -> InternalProgress {
        InternalProgress {
            start_time: TimePair::now(),
            chunk_size: 0,
            unfinished_chunks: vec![],
            current_chunks: BTreeMap::new(),
            //unpack_chunks: BTreeMap::new(),
            total_chunks: 0,
            total_download_size: None,
            total_downloaded: 0,
            chunk_downloaded: vec![],
            total_unpacked: 0,
            total_unpack_size: None,
            stop_requested: false,
            paused: false,
            progress_buckets_download: ProgressHistory::new(),
            progress_buckets_unpack: ProgressHistory::new(),
            finish_time: None,
            error_time: None,
            error_message: None,
            error_message_download: None,
            error_message_unpack: None,
            download_url: None,
            download_threads: 0,
            server_chunk_support: false,
            last_unpacked_files: VecDeque::new(),
            unpacked_files: 0,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurboDownloaderProgress {
    pub start_time: chrono::DateTime<Utc>,
    pub current_time: chrono::DateTime<chrono::Utc>,
    pub download_threads: usize,
    pub server_chunk_support: bool,
    pub chunk_size: usize,
    pub downloaded: usize,
    pub unpacked: usize,
    pub stop_requested: bool,
    pub paused: bool,
    pub elapsed_time_sec: f64,
    pub eta_sec: Option<u64>,
    pub finish_time: Option<chrono::DateTime<chrono::Utc>>,
    pub current_download_speed: usize,
    pub current_unpack_speed: usize,
    pub error_message: Option<String>,
    pub error_message_download: Option<String>,
    pub error_message_unpack: Option<String>,
    pub total_unpack_size: Option<usize>,
    pub total_download_size: Option<usize>,
    pub download_url: Option<String>,
    pub chunks_downloading: usize,
    pub chunks_total: usize,
    pub chunks_left: usize,
    pub current_chunks: BTreeMap<usize, DownloadChunkProgress>,
    pub unpacked_files: usize,
    pub last_unpacked_files: VecDeque<UnpackedFileInfo>,
    //pub unpack_chunks: BTreeMap<usize, UnpackChunkProgress>,
    //pub progress_buckets_download: ProgressHistory,
    //pub progress_buckets_unpack: ProgressHistory,
}

impl InternalProgress {
    pub fn progress(&self) -> TurboDownloaderProgress {
        TurboDownloaderProgress {
            start_time: self.start_time.to_utc().unwrap(),
            current_time: TimePair::now().to_utc().unwrap(),
            download_threads: self.download_threads,
            chunk_size: self.chunk_size,
            downloaded: self.total_downloaded + self.chunk_downloaded.iter().sum::<usize>(),
            unpacked: self.total_unpacked,
            stop_requested: self.stop_requested,
            paused: self.paused,
            elapsed_time_sec: self.get_elapsed().as_secs_f64(),
            eta_sec: self.get_time_left_sec(),
            finish_time: self.finish_time.as_ref().and_then(|ts| ts.to_utc().ok()),
            current_download_speed: self.progress_buckets_download.get_speed(),
            current_unpack_speed: self.progress_buckets_unpack.get_speed(),
            error_message: self.error_message.clone(),
            error_message_download: self.error_message_download.clone(),
            error_message_unpack: self.error_message_unpack.clone(),
            total_unpack_size: self.total_unpack_size,
            total_download_size: self.total_download_size,
            download_url: self.download_url.clone(),
            chunks_downloading: self.chunk_downloaded.len(),
            chunks_total: self.total_chunks,
            chunks_left: self.unfinished_chunks.len(),
            //progress_buckets_download: self.progress_buckets_download.clone(),
            //progress_buckets_unpack: self.progress_buckets_unpack.clone(),
            current_chunks: self.current_chunks.clone(),
            server_chunk_support: self.server_chunk_support,
            unpacked_files: self.unpacked_files,
            last_unpacked_files: self.last_unpacked_files.clone(),
            //unpack_chunks: self.unpack_chunks.clone(),
        }
    }

    pub fn get_elapsed(&self) -> time::Duration {
        self.finish_time
            .as_ref()
            .map(|t| t.as_ts())
            .unwrap_or_else(|| Instant::now())
            - self.start_time.as_ts()
    }

    pub fn get_time_left_sec(&self) -> Option<u64> {
        if self.finish_time.is_some() {
            return Some(0);
        }
        let download_speed = self.get_download_speed();
        if download_speed < 100 {
            return None;
        }
        if let Some(total_download_size) = self.total_download_size {
            let seconds_left = (total_download_size
                - self.total_downloaded
                - self.chunk_downloaded.iter().sum::<usize>())
                / download_speed;
            return Some(seconds_left as u64);
        }
        None
    }

    pub fn get_download_speed(&self) -> usize {
        if self.finish_time.is_some() {
            return 0;
        }
        let elapsed = self.get_elapsed();
        if elapsed.is_zero() {
            return 0;
        }
        let res_f64 = (self.total_downloaded + self.chunk_downloaded.iter().sum::<usize>()) as f64
            / elapsed.as_secs_f64();
        res_f64.round() as usize
    }
    pub fn get_unpack_speed(&self) -> usize {
        if self.finish_time.is_some() {
            return 0;
        }
        let elapsed = self.get_elapsed();
        if elapsed.is_zero() {
            return 0;
        }
        let speed_f64 = (self.total_unpacked) as f64 / elapsed.as_secs_f64();
        speed_f64.round() as usize
    }
}
