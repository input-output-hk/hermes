pub mod cardano_chain_follower;
mod monitor;
pub mod pallas;

use std::path::PathBuf;

pub struct BenchmarkParams {
    pub mithril_snapshot_path: PathBuf,
}
