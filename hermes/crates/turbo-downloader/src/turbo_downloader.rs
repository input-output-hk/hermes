use std::{
    fs,
    fs::File,
    path::{Path, PathBuf},
    sync::{mpsc::sync_channel, Arc, Mutex, MutexGuard},
    thread,
    time::Instant,
};

use anyhow::anyhow;
use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use lz4_flex::frame::FrameDecoder;
use tar::Archive;
use tracing::{debug, error, info};

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
    url: String,
    progress_context: Arc<Mutex<InternalProgress>>,
    options: TurboDownloaderOptions,
    download_started: bool,
    target_path: PathBuf,
    thread_last_stage: Option<thread::JoinHandle<()>>,
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

impl TurboDownloader {
    pub(crate) fn new(
        url: &str, target_path: PathBuf, turbo_downloader_options: TurboDownloaderOptions,
    ) -> Self {
        Self {
            url: url.to_string(),
            progress_context: Arc::new(Mutex::new(InternalProgress::default())),
            download_started: false,
            target_path,
            thread_last_stage: None,
            options: turbo_downloader_options,
        }
    }

    /// Process inputs and try to start download
    pub(crate) async fn start_download(self: &mut TurboDownloader) -> anyhow::Result<()> {
        if self.download_started {
            return Err(anyhow::anyhow!("Download already started"));
        }
        self.progress_context
            .lock()
            .expect("Failed to obtain lock")
            .start_time = TimePair::now();
        self.download_started = true;
        let url = self.url.clone();

        // Don't allow inferred target path,  must be specified.
        let target_path = self.target_path.clone();

        if !self.options.ignore_directory_exists && target_path.exists() {
            return Err(anyhow!(
                "Output directory from url already exists: {}. Remove it or specify --force flag",
                target_path.display()
            ));
        }

        info!("starting download...");
        let (send_download_chunks, receive_download_chunks) = sync_channel(1);

        let download_thread_count = self.options.download_threads;
        let download_loop_init_result = {
            let pc = self.progress_context.clone();
            let download_url = url.clone();
            let options = self.options.clone();
            let t = thread::spawn(move || {
                init_download_loop(download_thread_count, options, pc.clone(), &download_url)
            });

            loop {
                if t.is_finished() {
                    match t.join().unwrap() {
                        Ok(download_loop_init_result) => {
                            info!("Download loop initialized");
                            break download_loop_init_result;
                        },
                        Err(err) => {
                            error!("Error when initializing download: {:?}", err);
                            // stop other threads as well
                            return Err(err);
                        },
                    }
                } else {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        };

        let mut threads = Vec::new();

        for thread_no in 0..download_loop_init_result.threads_to_spawn {
            let pc = self.progress_context.clone();
            let options = self.options.clone();
            let send = send_download_chunks.clone();
            let download_loop_init_result = download_loop_init_result.clone();
            threads.push(thread::spawn(move || {
                match download_loop(
                    thread_no,
                    options,
                    pc.clone(),
                    send,
                    download_loop_init_result,
                ) {
                    Ok(_) => {
                        info!("Download loop finished, finishing thread");
                    },
                    Err(err) => {
                        error!("Error in download loop: {:?}, finishing thread", err);
                        // stop other threads as well
                        pc.lock().unwrap().stop_requested = true;
                        pc.lock().unwrap().error_message_download = Some(err.to_string());
                    },
                }
            }));
        }

        let mut p = MpscReaderFromReceiver::new(
            receive_download_chunks,
            false,
            self.progress_context.clone(),
            true,
        );

        let (send_unpack_chunks, receive_unpack_chunks) = sync_channel::<DataChunk>(1);

        let pc = self.progress_context.clone();
        let download_url = url.clone();
        let options = self.options.clone();
        let t2 = thread::spawn(move || {
            let download_url = match resolve_url(download_url, pc.clone()) {
                Ok(url) => url,
                Err(err) => {
                    pc.lock().unwrap().error_message_unpack = Some(err.to_string());
                    return;
                },
            };

            let res = if download_url.ends_with(".gz") {
                let mut gz = GzDecoder::new(&mut p);
                decode_loop(pc.clone(), &options, &mut gz, send_unpack_chunks)
            } else if download_url.ends_with(".lz4") {
                let mut lz4 = FrameDecoder::new(&mut p);
                decode_loop(pc.clone(), &options, &mut lz4, send_unpack_chunks)
            } else if download_url.ends_with(".bz2") {
                let mut bz2 = BzDecoder::new(&mut p);
                decode_loop(pc.clone(), &options, &mut bz2, send_unpack_chunks)
            } else if download_url.ends_with(".xz") {
                let mut xz_dec = xz2::read::XzDecoder::new(&mut p);
                decode_loop(pc.clone(), &options, &mut xz_dec, send_unpack_chunks)
            } else if download_url.ends_with(".zst") {
                let mut zstd_dec = zstd::stream::read::Decoder::new(&mut p).unwrap();
                decode_loop(pc.clone(), &options, &mut zstd_dec, send_unpack_chunks)
            } else {
                panic!("Unknown file type");
            };
            if let Err(err) = res {
                error!("Error in decode loop: {:?}, finishing thread", err);
                // stop other threads as well
                pc.lock().unwrap().stop_requested = true;
                pc.lock().unwrap().error_message_unpack = Some(err.to_string());
            };
            info!("Decode loop finished, finishing thread");
        });

        let mut p2 = MpscReaderFromReceiver::new(
            receive_unpack_chunks,
            false,
            self.progress_context.clone(),
            false,
        );

        let download_url = url;

        let pc = self.progress_context.clone();
        let options = self.options.clone();
        self.thread_last_stage = Some(thread::spawn(move || {
            let download_url = match resolve_url(download_url, pc.clone()) {
                Ok(url) => url,
                Err(err) => {
                    pc.lock().unwrap().error_message = Some(err.to_string());
                    return;
                },
            };

            let res = if download_url.contains(".tar.") {
                let mut archive = Archive::new(p2);

                match tar_unpack(&target_path, &mut archive, options, pc.clone()) {
                    Ok(_) => {
                        info!("Successfully unpacked");
                        Ok(())
                    },
                    Err(err) => {
                        error!("Error while unpacking {:?}", err);
                        Err(err)
                    },
                }

                // match archive.unpack(target_path) {
                // Ok(_) => {
                // info!("Successfully unpacked");
                // Ok(())
                // }
                // Err(err) => {
                // error!("Error while unpacking {:?}", err);
                // Err(err)
                // }
                // }
            } else {
                let mut output_file = File::create(&target_path).unwrap();
                match std::io::copy(&mut p2, &mut output_file) {
                    Ok(_) => {
                        info!("Successfully written file {:?}", target_path);
                        Ok(())
                    },
                    Err(err) => {
                        error!("Error while writing {:?}", err);
                        Err(err)
                    },
                }
            };
            match res {
                Ok(_) => {
                    pc.lock().unwrap().stop_requested = true;
                    for t1 in threads {
                        t1.join().unwrap();
                    }
                    t2.join().unwrap();
                    pc.lock().unwrap().finish_time = Some(TimePair::now());
                },
                Err(err) => {
                    pc.lock().unwrap().error_message = Some(format!("{err:?}"));
                    pc.lock().unwrap().stop_requested = true;
                    for t1 in threads {
                        t1.join().unwrap();
                    }
                    t2.join().unwrap();
                    pc.lock().unwrap().error_time = Some(Instant::now());
                },
            }
        }));

        Ok(())
    }

    /// Returns serializable [TurboDownloaderProgress] object
    pub fn get_progress(self: &TurboDownloader) -> TurboDownloaderProgress {
        self.get_progress_guard().progress()
    }

    /// Returns progress as human readable line
    pub fn get_progress_human_line(self: &TurboDownloader) -> String {
        let progress = self.get_progress_guard();

        let eta_string = if let Some(eta) = progress.get_time_left_sec() {
            let seconds = eta % 60;
            let minutes = (eta / 60) % 60;
            let hours = (eta / 60) / 60;
            format!("ETA: {hours:02}:{minutes:02}:{seconds:02}")
        } else {
            "ETA: unknown".to_string()
        };
        let percent_string = if let Some(total_length) = progress.total_download_size {
            format!(
                "[{:.2}%]",
                ((progress.total_downloaded + progress.chunk_downloaded.iter().sum::<usize>())
                    as f64
                    / total_length as f64)
                    * 100.0
            )
        } else {
            "".to_string()
        };

        format!(
            "Downloaded: {} [{}/s now: {}/s], Unpack: {} [{}/s now: {}/s] - {} {}",
            bytes_to_human(
                progress.total_downloaded + progress.chunk_downloaded.iter().sum::<usize>()
            ),
            bytes_to_human(progress.get_download_speed()),
            bytes_to_human(progress.progress_buckets_download.get_speed()),
            bytes_to_human(progress.total_unpacked),
            bytes_to_human(progress.get_unpack_speed()),
            bytes_to_human(progress.progress_buckets_unpack.get_speed()),
            eta_string,
            percent_string
        )
    }

    /// Cancel download
    pub fn signal_stop(self: &TurboDownloader) {
        let mut pc = self
            .progress_context
            .lock()
            .expect("Failed to lock progress context");
        pc.stop_requested = true;
    }

    /// Downloader supports pausing and resuming, you can call this method to pause
    /// download
    pub fn pause_download(self: &TurboDownloader) {
        let mut pc = self
            .progress_context
            .lock()
            .expect("Failed to lock progress context");
        pc.paused = true;
    }

    /// Downloader supports pausing and resuming, you can call this method to resume
    /// download
    pub fn resume_download(self: &TurboDownloader) {
        let mut pc = self
            .progress_context
            .lock()
            .expect("Failed to lock progress context");
        pc.paused = false;
    }

    /// Check if download is finished
    pub fn is_finished(self: &TurboDownloader) -> bool {
        if let Some(thread_last_stage) = self.thread_last_stage.as_ref() {
            thread_last_stage.is_finished()
        } else {
            false
        }
    }

    fn get_progress_guard(self: &TurboDownloader) -> MutexGuard<InternalProgress> {
        self.progress_context
            .lock()
            .expect("Failed to lock progress context")
    }

    /// Check if download is started
    pub fn is_started(self: &TurboDownloader) -> bool {
        self.download_started
    }
}
