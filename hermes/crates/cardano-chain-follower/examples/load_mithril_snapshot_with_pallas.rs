//! Load a Mithril Snapshot from file using `pallas`.
//!
//! This example requires a valid Mithril snapshot containing the genesis block in the
//! `examples/base_snapshot` directory.
use std::{
    cell::RefCell,
    error::Error,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use pallas::ledger::traverse::MultiEraBlock;
use pallas_hardano::storage::immutable::Point;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

/// Default nterval in seconds used to generate statistics.
const DEFAULT_BLOCK_INTERVAL_IN_SECS: u64 = 20;

/// Timer to keep track of block statistics
struct BlockTimer {
    /// `Instant` when the timer was created.
    start_time: Instant,
    /// Interval in seconds used to generate statistics. Must be greater than 0.
    stats_interval: u64,
    /// `Instant` when the interval timer was created or last reset.
    interval_time: RefCell<Instant>,
}

impl BlockTimer {
    /// Create a new block timer with the given optional interval.
    ///
    /// If the interval is not given, the timer will generate statistics every 20 seconds.
    /// If `stats_interval` is less than 1, it will be set to 1.
    fn new(val: Option<u64>) -> Self {
        let start_time = Instant::now();
        let interval_time = RefCell::new(Instant::now());
        let stats_interval = std::cmp::max(val.unwrap_or(DEFAULT_BLOCK_INTERVAL_IN_SECS), 1);
        Self {
            start_time,
            stats_interval,
            interval_time,
        }
    }

    /// Return the total time in seconds with millisecond precision as a string.
    fn total_time_str(&self) -> String {
        let elapsed = self.start_time.elapsed();
        format!("{}.{:03}", elapsed.as_secs(), elapsed.subsec_millis())
    }

    /// Return true if the interval has elapsed since the timer was created or last reset.
    fn has_interval_elapsed(&self) -> bool {
        self.interval_time.borrow().elapsed() >= Duration::from_secs(self.stats_interval)
    }

    /// Reset the interval timer.
    fn reset_interval(&self) {
        self.interval_time.replace(Instant::now());
    }
}

/// Print a table row line.
fn print_snapshot_separating_line() {
    tracing::info!(
        "|{:-^16}|{:-^16}|{:-^16}|{:-^16}|{:-^16}|{:-^20}|",
        "",
        "",
        "",
        "",
        "",
        ""
    );
}
/// Print a table row line for the summary.
fn print_summary_table_row_line() {
    tracing::info!("|{:-^16}|{:-^16}|{:-^16}|{:-^16}|", "", "", "", "");
}

/// Print header for snapshot table.
fn print_snapshot_header() {
    print_snapshot_separating_line();
    tracing::info!(
        "|{:^16}|{:^16}|{:^16}|{:^16}|{:^16}|{:^20}|",
        "total time (s)",
        "total blocks",
        "synced to slot",
        "slots per sec",
        "slots remaining",
        "est. secs remaining"
    );
    print_snapshot_separating_line();
}

/// Print header for summary table.
fn print_summary_header() {
    print_summary_table_row_line();
    tracing::info!("|{:^67}|", "SUMMARY");
    print_summary_table_row_line();
    tracing::info!(
        "|{:^16}|{:^16}|{:^16}|{:^16}|",
        "total time (s)",
        "total blocks",
        "synced to slot",
        "slots per sec"
    );
    print_summary_table_row_line();
}

/// Process block data from path until it is exhausted.
#[allow(clippy::unwrap_used)]
#[allow(clippy::cast_precision_loss)]
fn process_block_data(path: &Path, start_point: Point) -> Result<(), Box<dyn Error>> {
    let tip =
        pallas_hardano::storage::immutable::get_tip(path)?.ok_or("No tip found in the snapshot.");
    let tip_slot = tip?.slot_or_default();

    let mut block_data_iter =
        pallas_hardano::storage::immutable::read_blocks_from_point(path, start_point)?;
    let interval_duration = std::env::var("INTERVAL_DURATION")
        .map(|d| d.parse().unwrap())
        .ok();
    let block_timer = BlockTimer::new(interval_duration);

    let mut showing_table = false;
    let mut last_slot = 0;
    let mut last_interval_slot = 0;
    let mut blocks_seen = 0;

    tracing::info!("Processing Mithril Snapshot with Pallas");

    while let Some(Ok(block_data)) = block_data_iter.next() {
        // decode the block
        let block = MultiEraBlock::decode(&block_data)?;
        blocks_seen += 1;
        let current_slot = block.slot();

        if block_timer.has_interval_elapsed() {
            if !showing_table {
                showing_table = true;
                // print header
                print_snapshot_header();
            }
            let slots_in_interval = current_slot - last_interval_slot + 1;
            let slots_remaining = tip_slot - current_slot;
            let number_of_blocks_seen = blocks_seen;
            let slots_per_second = slots_in_interval as f64 / block_timer.stats_interval as f64;
            let secs_remaining = slots_remaining as f64 / slots_per_second.max(0.001);
            let total_time_str = block_timer.total_time_str();

            tracing::info!("|{total_time_str:>normal_w$} |{number_of_blocks_seen:>normal_w$} |{last_slot:>normal_w$} |{:>normal_w$} |{slots_remaining:>normal_w$} |{:>wider$} |", format!("{slots_per_second:.1}"), format!("{secs_remaining:.3 }" ), normal_w=15, wider=19);

            // Reset the interval timer
            block_timer.reset_interval();
            last_interval_slot = current_slot;
        }
        last_slot = current_slot;
    }

    if showing_table {
        print_snapshot_separating_line();
    }

    tracing::info!("\nFinished!\n");

    if last_slot == tip_slot {
        print_summary_header();
        let slots_per_second =
            (tip_slot) as f64 / block_timer.start_time.elapsed().as_secs() as f64;
        tracing::info!(
            "|{:^16}|{:^16}|{:^16}|{:^16}|",
            block_timer.total_time_str(),
            blocks_seen,
            last_slot,
            format!("{slots_per_second:.1}"),
        );
        print_summary_table_row_line();
    } else {
        tracing::error!("Incomplete snapshot. The last slot: {last_slot} does not match the expected slot: {tip_slot}");
    }

    Ok(())
}

/// Main test
fn main() -> Result<(), Box<dyn Error>> {
    // set the tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    // get path to snapshot
    let base_path = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let path = PathBuf::from(base_path).join("examples/base_snapshot");
    // set the start point to be the genesis block and process data.
    let start_point = Point::Origin;
    process_block_data(&path, start_point)?;
    // exit
    Ok(())
}
