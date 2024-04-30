//! Benchmark-specific implementations.

pub mod cardano_chain_follower;
mod monitor;
pub mod pallas;

use std::{
    collections::BinaryHeap,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use pallas_traverse::MultiEraBlock;

/// Parameters accepted by the benchmarks.
pub struct BenchmarkParams {
    /// Path to the Mithril snapshot that will be read by the benchmark.
    pub mithril_snapshot_path: PathBuf,
}

/// Locate the tip of a Mithril snapshot.
///
/// NOTE(fsgr):
/// This was needed because the current implementation found in pallas fails if
/// the Immutable DB indexes are corrupted. This seems to happen often since
/// Mithril snapshots are taken while the Cardano node is running. For this reason,
/// this function finds the most recent *valid* block in the snapshot in order to
/// get the benchmarks working for now.
fn snapshot_tip(path: &Path) -> anyhow::Result<Option<Vec<u8>>> {
    // First we collect all the .chunk files in an ordered manner.
    let mut chunk_files = BinaryHeap::new();

    for result in fs::read_dir(path)? {
        let entry = result?;

        let path = entry.path();

        match path.extension().map(OsStr::to_string_lossy) {
            None => continue,
            Some(ext) => {
                if ext != "chunk" {
                    continue;
                }
            },
        }

        if let Some(stem) = path.file_stem() {
            chunk_files.push(stem.to_string_lossy().to_string());
        }
    }

    while let Some(filename) = chunk_files.pop() {
        let reader = pallas_hardano::storage::immutable::chunk::read_blocks(path, &filename)?;

        if let Some(last_valid_block_data) = reader.map_while(Result::ok).last() {
            if MultiEraBlock::decode(&last_valid_block_data).is_ok() {
                return Ok(Some(last_valid_block_data));
            }
        }
    }

    Ok(None)
}
