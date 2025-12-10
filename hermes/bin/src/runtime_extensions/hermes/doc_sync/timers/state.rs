//! Timer state backing quiet keepalives and jitter scheduling.

use std::{sync::Arc, thread::JoinHandle};

use tokio::sync::{Mutex, Notify};

use super::config::SyncTimersConfig;

/// Keepalive callback
type KeepaliveCallback = Arc<dyn Fn() -> Result<(), anyhow::Error> + Send + Sync>;

/// State managing timers (Quiet period background task + helpers for one-off jitters).
pub struct SyncTimersState {
    /// Timer configuration
    cfg: SyncTimersConfig,
    /// Callback to invoke when a keepalive is sent.
    send_new_keepalive: KeepaliveCallback,
    /// Handle for the background quiet-period keepalive task.
    keepalive_task: Mutex<Option<JoinHandle<()>>>,
    /// Notification for resetting the quiet timer.
    reset_new_notify: Notify,
}

impl SyncTimersState {
    /// Create a timer state.
    pub fn new(
        cfg: SyncTimersConfig,
        send_new_keepalive: KeepaliveCallback,
    ) -> Arc<Self> {
        Arc::new(Self {
            cfg,
            send_new_keepalive,
            keepalive_task: Mutex::new(None),
            reset_new_notify: Notify::new(),
        })
    }

    /// Async sleep for the .syn jitter duration.
    /// Call before sending .syn.
    #[allow(dead_code)]
    pub async fn wait_syn_backoff(&self) {
        let dur = self.cfg.random_syn_jitter();
        tokio::time::sleep(dur).await;
    }

    /// Async sleep for the .dif (or .prv) responder jitter duration.
    /// Call before publishing .dif/.prv.
    #[allow(dead_code)]
    pub async fn wait_responder_jitter(&self) {
        let dur = self.cfg.random_responder_jitter();
        tokio::time::sleep(dur).await;
    }

    /// Start the quiet period keepalive timer for .new topic
    pub fn start_quiet_timer(self: &Arc<Self>) {
        // Check whether the task is already running
        let mut task_guard = self.keepalive_task.blocking_lock();
        if task_guard.is_some() {
            tracing::trace!("Quiet timer already running");
            return;
        }

        let this = Arc::clone(self);
        let handle = std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
            {
                Ok(rt) => rt,
                Err(err) => {
                    tracing::error!("Failed to build Tokio runtime for quiet timer: {err}");
                    return;
                },
            };
            rt.block_on(async move {
                this.run_quiet_timer().await;
            });
        });
        *task_guard = Some(handle);
    }

    /// Background loop that waits for the quiet period, emits keepalives, and listen to reset notifications.
    async fn run_quiet_timer(&self) {
        loop {
            // Pick a random duration for THIS cycle
            let sleep_dur = self.cfg.random_quiet();

            tokio::select! {
                // Case A: Timer expired (send keepalive)
                () = tokio::time::sleep(sleep_dur) => {
                    // Using spawn_blocking because the callback may perform blocking operations
                    let result = tokio::task::spawn_blocking({
                        let callback = self.send_new_keepalive.clone();
                        move || callback()
                    }).await;

                    match result {
                        Ok(Ok(())) => tracing::debug!("Keepalive sent"),
                        Ok(Err(e)) => tracing::warn!("Keepalive failed: {e:?}"),
                        Err(e) => tracing::error!("Keepalive task failed: {e:?}"),
                    }
                    // Loop restarts -> New random duration picked
                }
                // Case B: Timer reset triggered (.new observed externally)
                () = self.reset_new_notify.notified() => {
                    tracing::trace!("Quiet timer reset");
                    // Loop restarts immediately -> New random duration picked
                }
            }
        }
    }

    /// Reset quiet-period timer (call on every received or posted .new)
    pub fn reset_quiet_timer(&self) {
        tracing::trace!("Resetting quiet timer");
        // Notify the background task to reset its sleep loop
        self.reset_new_notify.notify_waiters();
    }

    /// Stop the quiet-period background task, if running.
    pub fn stop_quiet_timer(&self) {
        self.keepalive_task.blocking_lock().take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    fn timer_config() -> SyncTimersConfig {
        SyncTimersConfig {
            quiet_period: Duration::from_millis(100)..Duration::from_millis(100),
            ..Default::default()
        }
    }
    #[tokio::test]
    async fn test_start_quiet_timer_count_keepalive() {
        // Track how many times the callback is called
        let callback_count = Arc::new(AtomicU32::new(0)).clone();
        let callback: KeepaliveCallback = Arc::new({
            let counter = callback_count.clone();
            move || {
                counter.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
        });

        let state = SyncTimersState::new(timer_config(), callback);
        let state_clone = state.clone();
        tokio::task::spawn_blocking(move || {
            state_clone.start_quiet_timer();
        })
        .await
        .unwrap();

        // Trigger 2 keepalives
        tokio::time::sleep(Duration::from_millis(250)).await;
        state.reset_quiet_timer();
        // Trigger 1 keepalive
        tokio::time::sleep(Duration::from_millis(120)).await;

        let count = callback_count.load(Ordering::Relaxed);
        assert!(
            count == 3,
            "Expected callback to be called 3 times, got {count}"
        );
    }
}
