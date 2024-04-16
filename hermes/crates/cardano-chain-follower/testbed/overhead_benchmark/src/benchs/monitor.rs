use std::time::{Duration, Instant};

use tokio::sync::mpsc;

#[derive(Debug, Default, Clone, Copy)]
pub struct BenchmarkStats {
    pub blocks_read: u64,
    pub block_bytes_read: u64,
    pub current_block: u64,
    pub current_slot: u64,
}

impl BenchmarkStats {
    fn apply_partial_update(&mut self, u: &BenchmarkStats) {
        self.blocks_read += u.blocks_read;
        self.block_bytes_read += u.block_bytes_read;
        self.current_block = u.current_block;
        self.current_slot = u.current_slot;
    }
}

pub struct Handle {
    join_handle: tokio::task::JoinHandle<()>,
    update_tx: mpsc::UnboundedSender<BenchmarkStats>,
}

impl Handle {
    pub async fn send_update(&self, b: BenchmarkStats) -> anyhow::Result<()> {
        self.update_tx.send(b)?;
        Ok(())
    }

    pub async fn close(self) {
        // Drop update sender in order to cancel the task
        drop(self.update_tx);
        // Wait the task to finish
        drop(self.join_handle.await);
    }
}

pub fn spawn_task() -> Handle {
    let (update_tx, update_rx) = mpsc::unbounded_channel();

    let join_handle = tokio::spawn(monitor_task(update_rx));

    Handle {
        join_handle,
        update_tx,
    }
}

struct MonitorTaskState {
    stats_buffer: BenchmarkStats,
    overall_stats: BenchmarkStats,
    started_at: Instant,
    last_checkpoint: Instant,
}

impl MonitorTaskState {
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
            "{: ^10} | {: ^10} | {: ^10} | {: ^10} | {: ^15} | {: ^20} | {: ^20}",
            "TOTAL TIME",
            "BLOCK",
            "SLOT",
            "BLOCKS/s",
            "BLOCKS MB/s",
            "BLOCKS/s (inter)",
            "BLOCKS MB/s (inter)"
        );
        println!(
            "{:-<10} + {:-<10} + {:-<10} + {:-<10} + {:-<15} + {:-<20} + {:-<20}",
            "", "", "", "", "", "", ""
        );
        println!(
            "{: ^10} | {: ^10} | {: ^10} | {: ^10.2} | {: ^15.2} | {: ^20.2} | {: ^20.2}",
            elapsed_str,
            self.overall_stats.current_block,
            self.overall_stats.current_slot,
            blocks_per_sec,
            block_mb_per_sec,
            blocks_per_sec_interval,
            block_mb_per_sec_interval
        );
        println!();

        self.stats_buffer = BenchmarkStats::default();
        self.last_checkpoint = Instant::now();
    }
}

async fn monitor_task(mut update_rx: mpsc::UnboundedReceiver<BenchmarkStats>) {
    let mut ticker = tokio::time::interval(Duration::from_secs(1));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let mut state = MonitorTaskState {
        stats_buffer: BenchmarkStats::default(),
        overall_stats: BenchmarkStats::default(),
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
