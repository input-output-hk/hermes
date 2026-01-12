//! Timer configuration definitions (jitter ranges, quiet-period bounds).
//! The configuration is derived from: <https://github.com/input-output-hk/hermes/blob/main/docs/src/architecture/08_concepts/document_sync/protocol_spec.md#timers-and-retries>

use std::{convert::TryFrom, ops::Range, time::Duration};

use rand::Rng;

/// Minimum backoff/jitter before sending .syn: uniform random in ms.
const T_SYN_MIN_MS: u64 = 200;
/// Maximum backoff/jitter before sending .syn: uniform random in ms.
const T_SYN_MAX_MS: u64 = 800;

/// Default range backoff/jitter before sending .syn.
/// Spec: [200ms, 800ms].
const T_SYN_RANGE: Range<Duration> =
    Duration::from_millis(T_SYN_MIN_MS)..Duration::from_millis(T_SYN_MAX_MS);

/// Minimum responder jitter before publishing .dif (and .prv) in ms.
const R_MIN_MS: u64 = 50;
/// Maximum responder jitter before publishing .dif (and .prv) in ms.
const R_MAX_MS: u64 = 250;

/// Default responder jitter range.
/// Spec: [50ms, 250ms].
const RESPONDER_RANGE: Range<Duration> =
    Duration::from_millis(R_MIN_MS)..Duration::from_millis(R_MAX_MS);

/// Minimum quiet period in seconds for .new re-announce.
const T_QUIET_MIN_SEC: u64 = 20;
/// Maximum quiet period in seconds for .new re-announce.
const T_QUIET_MAX_SEC: u64 = 60;

/// Default quiet period range.
/// Spec: [20s, 60s].
const T_QUIET_RANGE: Range<Duration> =
    Duration::from_secs(T_QUIET_MIN_SEC)..Duration::from_secs(T_QUIET_MAX_SEC);

/// Configuration for all Document Sync timers per topic channel.
#[derive(Debug, Clone)]
pub struct SyncTimersConfig {
    /// Jitter range for sending .syn
    pub syn_jitter: Range<Duration>,
    /// Jitter range for responding with .dif (or .prv)
    pub responder_jitter: Range<Duration>,
    /// Quiet period re-announcement range for .new
    pub quiet_period: Range<Duration>,
}

impl Default for SyncTimersConfig {
    fn default() -> Self {
        Self {
            syn_jitter: T_SYN_RANGE,
            responder_jitter: RESPONDER_RANGE,
            quiet_period: T_QUIET_RANGE,
        }
    }
}

impl SyncTimersConfig {
    /// Generates backoff/jitter for .syn
    #[must_use]
    pub fn random_syn_jitter(&self) -> Duration {
        Self::random_duration(&self.syn_jitter)
    }

    /// Generates responder jitter for .dif (and .prv)
    #[must_use]
    pub fn random_responder_jitter(&self) -> Duration {
        Self::random_duration(&self.responder_jitter)
    }

    /// Generates quiet period
    #[must_use]
    pub fn random_quiet(&self) -> Duration {
        Self::random_duration(&self.quiet_period)
    }

    /// Helper to generate a random duration within a specific Range.
    fn random_duration(range: &Range<Duration>) -> Duration {
        let min_ms = Self::duration_millis(range.start);
        let max_ms = Self::duration_millis(range.end);

        if min_ms >= max_ms {
            return range.start;
        }

        let millis = rand::rng().random_range(min_ms..max_ms);
        Duration::from_millis(millis)
    }

    /// Duration converter to `u64` milliseconds.
    fn duration_millis(duration: Duration) -> u64 {
        u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
    }
}
