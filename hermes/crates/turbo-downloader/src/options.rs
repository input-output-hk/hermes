use std::path::PathBuf;

use crate::TurboDownloader;

/// Turbo Downloader Options.
#[derive(Debug, Clone)]
pub struct TurboDownloaderOptions {
    /// Maximum size of the download buffer (bytes).
    pub dl_buffer_size: u64,
    /// Size of the chunks within the download buffer (bytes).
    pub dl_chunk_size: u64,
    /// Ignore symlinks when un-taring
    pub ignore_symlinks: bool,
    /// Ignore directory exists error
    pub ignore_directory_exists: bool,
}

impl Default for TurboDownloaderOptions {
    fn default() -> Self {
        let dl_buffer_size = 64 * 1024 * 1024; // 64MB Download Buffer
        let dl_chunk_size = dl_buffer_size / 16; // 16 Chunk Downloader's = this size of a chunk.
        Self {
            dl_buffer_size,
            dl_chunk_size,
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
