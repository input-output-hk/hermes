//! Monitor task module.
//!
//! The monitoring task is responsible for receiving statistics from the benchmarks
//! and calculating metrics and displaying them.
#![allow(clippy::cast_precision_loss)]

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

/// Statistics measured by benchmarks.
#[derive(Debug, Default, Clone, Copy)]
pub struct BenchmarkStats {
    /// Blocks read by the benchmark.
    pub blocks_read: u64,
    /// The amount of block data bytes read by the benchmark.
    pub block_bytes_read: u64,
    /// Current block the benchmark is at.
    pub current_block: u64,
    /// Current slot the benchmark is at.
    pub current_slot: u64,
}

impl BenchmarkStats {
    /// Adds counters from another stats value and sets gauge values to the
    /// values from the stats update.
    fn apply_partial_update(&mut self, u: &BenchmarkStats) {
        self.blocks_read += u.blocks_read;
        self.block_bytes_read += u.block_bytes_read;
        self.current_block = u.current_block;
        self.current_slot = u.current_slot;
    }
}

/// Monitoring task handle.
pub struct Handle {
    /// Join handle used to wait the completion of the task when closing.
    join_handle: tokio::task::JoinHandle<()>,
    /// Stats update channel sender.
    update_tx: mpsc::UnboundedSender<BenchmarkStats>,
}

impl Handle {
    /// Sends a stats update to the monitoring task.
    pub fn send_update(&self, b: BenchmarkStats) -> anyhow::Result<()> {
        self.update_tx.send(b)?;
        Ok(())
    }

    /// Closes the monitor task.
    ///
    /// This means waiting for it to print the latest statistics.
    pub async fn close(self) {
        // Drop update sender in order to cancel the task
        drop(self.update_tx);
        // Wait the task to finish
        drop(self.join_handle.await);
    }
}

/// Spawns a monitoring task.
pub fn spawn_task(last_block_number: u64) -> Handle {
    let (update_tx, update_rx) = mpsc::unbounded_channel();

    let join_handle = tokio::spawn(monitor_task(update_rx, last_block_number));

    Handle {
        join_handle,
        update_tx,
    }
}

/// Monitoring task state.
struct MonitorTaskState {
    /// Buffered stats used to calculate metrics between reports.
    stats_buffer: BenchmarkStats,
    /// Stats accumulated since the start of a benchmark.
    overall_stats: BenchmarkStats,
    /// Last block that the benchmark will read.
    /// Used to calculate the benchmark progress.
    last_block_number: u64,
    /// Instant at which the monitoring task was started.
    started_at: Instant,
    ///  Instant at which the monitoring task has reported stats the last.
    last_checkpoint: Instant,
}

impl MonitorTaskState {
    /// Calculates and reports benchmark stats and update the overall stats.
    fn checkpoint(&mut self) {
        let total_elapsed = self.started_at.elapsed();
        let interval_elapsed = self.last_checkpoint.elapsed().as_secs_f64();

        self.overall_stats.apply_partial_update(&self.stats_buffer);

        let blocks_per_sec_interval = self.stats_buffer.blocks_read as f64 / interval_elapsed;
        let block_mb_per_sec_interval =
            self.stats_buffer.block_bytes_read as f64 / (1024.0 * 1024.0) / interval_elapsed;

        let blocks_per_sec = self.overall_stats.blocks_read as f64 / total_elapsed.as_secs_f64();
        let block_mb_per_sec = self.overall_stats.block_bytes_read as f64
            / (1024.0 * 1024.0)
            / total_elapsed.as_secs_f64();

        let elapsed_secs = total_elapsed.as_secs();
        let elapsed_str = format!(
            "{2:0>2}:{1:0>2}:{0:0>2}",
            elapsed_secs % 60,
            (elapsed_secs / 60) % 60,
            elapsed_secs / 60 / 60
        );

        println!(
            "{: ^10} | {: ^10} | {: ^10} | {: ^10} | {: ^15} | {: ^20} | {: ^20} | {: ^10}",
            "TOTAL TIME",
            "BLOCK",
            "SLOT",
            "BLOCKS/s",
            "BLOCKS MB/s",
            "BLOCKS/s (inter)",
            "BLOCKS MB/s (inter)",
            "PROGRESS",
        );
        println!(
            "{:-<10} + {:-<10} + {:-<10} + {:-<10} + {:-<15} + {:-<20} + {:-<20} + {:-<10}",
            "", "", "", "", "", "", "", ""
        );
        println!(
            "{: ^10} | {: ^10} | {: ^10} | {: ^10.2} | {: ^15.2} | {: ^20.2} | {: ^20.2} | {: ^10}",
            elapsed_str,
            self.overall_stats.current_block,
            self.overall_stats.current_slot,
            blocks_per_sec,
            block_mb_per_sec,
            blocks_per_sec_interval,
            block_mb_per_sec_interval,
            format!(
                "{:.2}%",
                100.0 * self.overall_stats.current_block as f64 / self.last_block_number as f64
            ),
        );
        println!();

        self.stats_buffer = BenchmarkStats::default();
        self.last_checkpoint = Instant::now();
    }
}

/// Receives benchmark stats updates and reports them every second.
async fn monitor_task(
    mut update_rx: mpsc::UnboundedReceiver<BenchmarkStats>, last_block_number: u64,
) {
    let mut ticker = tokio::time::interval(Duration::from_secs(1));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let mut state = MonitorTaskState {
        stats_buffer: BenchmarkStats::default(),
        overall_stats: BenchmarkStats::default(),
        last_block_number,
        started_at: Instant::now(),
        last_checkpoint: Instant::now(),
    };

    'task_loop: loop {
        tokio::select! {
            _ = ticker.tick() => {
                state.checkpoint();
            }

            stats_update = update_rx.recv() => {
                let Some(stats_update) = stats_update else {
                    break 'task_loop;
                };

                state.stats_buffer.apply_partial_update(&stats_update);
            }
        }
    }

    // Print metrics one last time because we might have new data
    state.checkpoint();
}
