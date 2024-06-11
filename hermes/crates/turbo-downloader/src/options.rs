use std::path::PathBuf;

use crate::TurboDownloader;

/// Turbo Downloader Options.
#[derive(Debug, Clone)]
pub struct TurboDownloaderOptions {
    /// Size of download buffer in bytes, if memory is an issue, reduce this value
    /// If the download is slow, you can use smaller value and increase download threads.
    /// For the fast downloads buffer should be big to improve performance.
    pub chunk_size_downloader: usize,
    /// Size of the buffer used to decode the file
    pub chunk_size_decoder: usize,
    /// Limit speed per thread if needed
    pub max_download_speed: Option<usize>,
    /// Do not use CONTENT_RANGE header
    pub force_no_chunks: bool,
    /// Number of download threads/connections
    /// You can improve download speed by increasing this number,
    /// note that this will also increase memory usage
    pub download_threads: usize,
    /// Ignore symlinks when un-taring
    pub ignore_symlinks: bool,
    /// Ignore directory exists error
    pub ignore_directory_exists: bool,
}

impl Default for TurboDownloaderOptions {
    fn default() -> Self {
        let dl_threads = 20;
        let dl_buffer = 512 * 1024 * 1024;
        Self {
            chunk_size_downloader: dl_buffer / dl_threads,
            chunk_size_decoder: dl_buffer / dl_threads / 8,
            download_threads: dl_threads,
            max_download_speed: None,
            force_no_chunks: false,
            ignore_symlinks: false,
            ignore_directory_exists: true, // Default to safe over-write.
        }
    }
}

impl TurboDownloaderOptions {
    /// Constructs downloader from given options.
    pub async fn start_download(
        &self, url: &str, target_path: PathBuf,
    ) -> anyhow::Result<TurboDownloader> {
        let mut dl = TurboDownloader::new(url, target_path, self.clone());
        dl.start_download().await?;
        Ok(dl)
    }
}
