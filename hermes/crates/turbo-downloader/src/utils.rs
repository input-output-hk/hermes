use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use humansize::{FormatSizeOptions, SizeFormatter, DECIMAL};

use crate::progress::InternalProgress;

pub fn bytes_to_human(bytes: usize) -> SizeFormatter<usize, FormatSizeOptions> {
    SizeFormatter::new(bytes, DECIMAL)
}

pub fn resolve_url(
    download_url: String, pc: Arc<Mutex<InternalProgress>>,
) -> anyhow::Result<String> {
    if download_url.ends_with(".link") {
        loop {
            if let Some(url) = pc.lock().unwrap().download_url.clone() {
                break Ok(url);
            }
            if pc.lock().unwrap().stop_requested {
                return Err(anyhow::anyhow!("Stop requested"));
            }
            thread::sleep(Duration::from_secs(1));
        }
    } else {
        Ok(download_url)
    }
}
