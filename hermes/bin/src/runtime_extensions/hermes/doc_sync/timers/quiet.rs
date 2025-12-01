//! Quiet timer management for Document Sync protocol.
//!
//! Specification and terminology is defined:
//! <https://github.com/input-output-hk/hermes/blob/main/docs/src/architecture/08_concepts/document_sync/protocol_spec.md#timers-and-retries>

use std::{sync::Arc, time::Duration};

use tokio::{
    sync::{Mutex, Notify},
    task::JoinHandle,
    time::Instant,
};

use crate::{app::ApplicationName, ipfs::hermes_ipfs_publish};

/// Minimum quiet period in seconds
const T_QUIET_MIN: u64 = 20;
/// Maximum quiet period in seconds
const T_QUIET_MAX: u64 = 60;

/// Configuration for quiet timers per topic channel.
#[derive(Debug, Clone)]
pub struct QuietTimersConfig {
    // Quiet period re-announcement for .new
    pub t_quiet_min: Duration,
    pub t_quiet_max: Duration,
}

impl Default for QuietTimersConfig {
    fn default() -> Self {
        Self {
            t_quiet_min: Duration::from_secs(T_QUIET_MIN),
            t_quiet_max: Duration::from_secs(T_QUIET_MAX),
        }
    }
}

impl QuietTimersConfig {
    /// Tquiet uniformly random within [T_QUIET_MIN, T_QUIET_MAX]
    fn random_quiet(&self) -> Duration {
        let t_quiet = rand::random_range(self.t_quiet_min..=self.t_quiet_max);
        Duration::from_secs(t_quiet.as_secs())
    }
}

/// Timer state per channel
pub struct QuietTimersState {
    /// Timer configuration
    pub cfg: QuietTimersConfig,
    /// Last `.new` topic received
    pub last_new_received: Mutex<Instant>,
    /// Handle for the background keepalive task.
    pub keepalive_task: Mutex<Option<JoinHandle<()>>>,
    /// Notification for resetting the timer
    pub reset_new_notify: Notify,
    /// Application name
    pub app_name: ApplicationName,
    /// Channel topic
    pub channel_topic: String,
}

impl QuietTimersState {
    pub fn new(
        cfg: QuietTimersConfig,
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

    /// Start the quiet period keepalive timer for .new topic
    pub fn start_quiet_timer(&self) -> anyhow::Result<()> {
        todo!()
    }

    async fn run_quiet_timer(self) -> anyhow::Result<()> {
        loop {
            let sleep_dur = self.cfg.random_quiet();

            tokio::select! {
                // When timer expire, send keepalive
                _ = tokio::time::sleep(sleep_dur) => {
                    if let Err(err) = self.send_new_keepalive() {
                       tracing::warn!("Failed to send .new keepalive: {:?}", err);
                    }
                }
                // Notify that a new topic is received
                _ = self.reset_new_notify.notified() => {
                    continue
                }
            }
        }
    }

    fn send_new_keepalive(&self) -> anyhow::Result<()> {
        // TODO - implement a proper keepalive payload
        let payload = vec![];
        hermes_ipfs_publish(&self.app_name, &self.channel_topic, payload)
            .map_err(anyhow::Error::msg)?;
        Ok(())
    }

    /// Reset quiet-period timer (call on every received or posted .new)
    pub async fn reset_quiet_timer(&self) {
        todo!()
    }
}
