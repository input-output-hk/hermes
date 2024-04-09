//! Test
use std::{
    error::Error,
    path::PathBuf,
    time::{Duration, Instant},
};

use pallas::ledger::traverse::MultiEraBlock;
use pallas_hardano::storage::immutable::Point;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

/// Main test
#[allow(clippy::unwrap_used)]
fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
    let path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?).join("examples/base_snapshot");
    let tip = pallas_hardano::storage::immutable::get_tip(&path)?.ok_or("bad");
    println!("{path:?} {tip:?}");

    let start_point = Point::Origin;
    let start_time = Instant::now();
    let total_slots = tip.unwrap().slot_or_default();
    let mut block_data_iter =
        pallas_hardano::storage::immutable::read_blocks_from_point(&path, start_point)?;

    let timeout_duration = Duration::from_secs(10);

    let mut timer = Instant::now();
    let mut slots_seen = 0;
    while let Some(Ok(block_data)) = block_data_iter.next() {
        let block = MultiEraBlock::decode(&block_data)?;
        slots_seen += 1;
        if timer.elapsed() >= timeout_duration {
            let total_duration = start_time.elapsed().as_secs();
            tracing::info!(
                slots_seen = slots_seen,
                total_time = total_duration,
                "NUMBER={} SLOT={} HASH={}",
                block.number(),
                block.slot(),
                hex::encode(block.hash())
            );
            timer = Instant::now();
        }
    }
    tracing::info!(
        slots_seen = slots_seen,
        total_slots_expected = total_slots,
        total_time = start_time.elapsed().as_secs(),
        "SUMMARY"
    );
    Ok(())
}
