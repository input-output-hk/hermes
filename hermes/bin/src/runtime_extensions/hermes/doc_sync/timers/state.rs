//! Runtime state backing quiet keepalives and jitter scheduling.

use std::sync::Arc;

use tokio::{
    sync::{Mutex, Notify},
    task::JoinHandle,
    time::Instant,
};

use super::config::SyncTimersConfig;
use crate::{app::ApplicationName, ipfs::hermes_ipfs_publish};

/// Sentinel payload published during quiet-period keepalives.
const KEEPALIVE_SENTINEL: &[u8] = b"doc-sync-keepalive";

/// State managing timers (Quiet period background task + helpers for one-off jitters)
pub struct SyncTimersState {
    /// Timer configuration
    pub cfg: SyncTimersConfig,
    /// Last `.new` topic received (used for Quiet Timer metrics/logic)
    pub last_new_received: Mutex<Instant>,
    /// Handle for the background quiet-period keepalive task.
    pub keepalive_task: Mutex<Option<JoinHandle<()>>>,
    /// Notification for resetting the quiet timer
    pub reset_new_notify: Notify,
    /// Application name
    pub app_name: ApplicationName,
    /// Channel topic
    pub channel_topic: String,
}

impl SyncTimersState {
    /// Create a timer state for the supplied application/channel pair.
    pub fn new(
        cfg: SyncTimersConfig,
        app_name: ApplicationName,
        channel_topic: &str,
    ) -> Arc<Self> {
        Arc::new(Self {
            cfg,
            last_new_received: Mutex::new(Instant::now()),
            keepalive_task: Mutex::new(None),
            reset_new_notify: Notify::new(),
            app_name,
            channel_topic: channel_topic.to_string(),
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
        let mut task_guard = self.keepalive_task.blocking_lock();

        if task_guard.is_some() {
            return;
        }

        let this = self.clone();
        let handle = tokio::spawn(this.run_quiet_timer());

        *task_guard = Some(handle);
    }

    /// Background loop that waits for the quiet period and emits keepalives.
    async fn run_quiet_timer(self: Arc<Self>) {
        loop {
            // Pick a random duration for THIS cycle
            let sleep_dur = self.cfg.random_quiet();

            tokio::select! {
                // Case A: Timer expired (send keepalive)
                () = tokio::time::sleep(sleep_dur) => {
                    tracing::debug!(
                        "Quiet period {:?} elapsed for topic {}, sending keepalive",
                        sleep_dur,
                        self.channel_topic
                    );

                    if let Err(err) = self.send_new_keepalive() {
                       tracing::warn!("Failed to send .new keepalive: {:?}", err);
                    }
                    // Loop restarts -> New random duration picked
                }

                // Case B: Reset triggered (.new observed externally)
                () = self.reset_new_notify.notified() => {
                    tracing::trace!("Quiet timer reset received for topic {}", self.channel_topic);
                    // Loop restarts immediately -> New random duration picked
                    // This acts as the "timer reset"
                }
            }
        }
    }

    /// Publish a sentinel `.new` keepalive payload and reset the timer.
    fn send_new_keepalive(&self) -> anyhow::Result<()> {
        // TODO: Replace sentinel payload with deterministic CBOR envelope once SMT summary is
        // wired.
        hermes_ipfs_publish(
            &self.app_name,
            &self.channel_topic,
            KEEPALIVE_SENTINEL.to_vec(),
        )
        .map_err(|err| anyhow::Error::msg(format!("keepalive publish failed: {err:?}")))?;
        self.reset_quiet_timer();
        Ok(())
    }

    /// Reset quiet-period timer (call on every received or posted .new)
    pub fn reset_quiet_timer(&self) {
        {
            let mut last = self.last_new_received.blocking_lock();
            *last = Instant::now();
        }
        // Wake up the background task to reset its sleep loop
        self.reset_new_notify.notify_waiters();
    }

    /// Stop the quiet-period background task, if running.
    pub fn stop_quiet_timer(&self) {
        if let Some(handle) = self.keepalive_task.blocking_lock().take() {
            handle.abort();
        }
    }
}
