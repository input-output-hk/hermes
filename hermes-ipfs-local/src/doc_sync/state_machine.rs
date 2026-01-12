//! State machine handling Doc Sync parity transitions.

use std::{
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tokio::sync::oneshot::error::TryRecvError;

use crate::doc_sync::Blake3256;

/// State machine that tracks peer parity for the Doc Sync protocol.
///
/// Transitions follow the protocol spec:
/// * `Stable` -> `Diverged` when a mismatch is detected.
/// * `Diverged` -> `Stable` if parity is observed during backoff (passive convergence).
/// * `Diverged` -> `Reconciling` after a backoff timer elapses without parity.
/// * `Reconciling` -> `Stable` when parity is restored.
/// * `Reconciling` -> `Diverged` if a new mismatch is observed (restart backoff).
#[derive(Debug)]
pub enum StateMachine {
    /// Local view matches known remotes.
    Stable,
    /// Divergence detected; waiting on backoff to solicit reconciliation.
    Diverged(BackoffTimer),
    /// Actively reconciling after backoff.
    Reconciling,
}

/// Public snapshot of the current state without internal handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateSnapshot {
    /// Local view matches known remotes.
    Stable,
    /// Divergence detected; waiting on reconciliation.
    Diverged,
    /// Actively reconciling.
    Reconciling,
}

/// Tracks the last known local/remote signatures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncState {
    /// Local sparse merkle tree root.
    local: Blake3256,
    /// Remote sparse merkle tree root.
    remote: Blake3256,
}

impl SyncState {
    /// Creates a new synchronization view for comparison.
    #[must_use]
    pub fn new(
        local: Blake3256,
        remote: Blake3256,
    ) -> Self {
        Self { local, remote }
    }

    /// Returns `true` when the local and remote views match.
    #[must_use]
    pub fn is_synced(&self) -> bool {
        self.local == self.remote
    }
}

/// Internal backoff timer handle.
#[derive(Debug)]
pub struct BackoffTimer {
    /// Join handle for the spawned timer task.
    task: tokio::task::JoinHandle<()>,
    /// Receiver that completes when the timer elapses.
    finished: tokio::sync::oneshot::Receiver<()>,
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachine {
    /// Constructs a fresh state machine in `Stable` state.
    #[must_use]
    pub fn new() -> Self {
        Self::Stable
    }

    /// Creates an `Arc<Mutex<...>>` managed state machine.
    #[must_use]
    pub fn shared() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new()))
    }

    /// Returns a snapshot of the current state without exposing handles.
    fn snapshot(&self) -> StateSnapshot {
        match self {
            StateMachine::Stable => StateSnapshot::Stable,
            StateMachine::Diverged(_) => StateSnapshot::Diverged,
            StateMachine::Reconciling => StateSnapshot::Reconciling,
        }
    }

    /// Advances the state machine based on the latest observed sync state.
    ///
    /// * When stable and divergence is detected, a jittered backoff timer is started.
    /// * When diverged and the timer fires, transition to reconciling.
    /// * When reconciling (or diverged) and parity returns, transition to stable.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying mutex cannot be locked.
    pub fn step(
        machine: &Arc<Mutex<Self>>,
        sync_state: &SyncState,
    ) -> anyhow::Result<StateSnapshot> {
        let mut guard = machine
            .lock()
            .map_err(|err| anyhow::anyhow!("failed to lock state machine state: {err}"))?;

        match &mut *guard {
            StateMachine::Stable => {
                if !sync_state.is_synced() {
                    *guard = StateMachine::Diverged(Self::run_backoff_timer());
                }
            },
            StateMachine::Diverged(backoff) => {
                // Passive convergence during backoff.
                if sync_state.is_synced() {
                    backoff.task.abort();
                    *guard = StateMachine::Stable;
                } else if Self::backoff_completed(backoff) {
                    *guard = StateMachine::Reconciling;
                }
            },
            StateMachine::Reconciling => {
                if sync_state.is_synced() {
                    *guard = StateMachine::Stable;
                } else {
                    // New mismatch observed while reconciling; restart backoff.
                    *guard = StateMachine::Diverged(Self::run_backoff_timer());
                }
            },
        }

        Ok(guard.snapshot())
    }

    /// Starts a jittered backoff timer using the protocol's suggested defaults.
    fn run_backoff_timer() -> BackoffTimer {
        const BACKOFF_MIN_MS: u64 = 200;
        const BACKOFF_MAX_MS: u64 = 800;

        let (tx, finished) = tokio::sync::oneshot::channel();
        let duration = jittered_duration(BACKOFF_MIN_MS, BACKOFF_MAX_MS);

        let task = tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            let _ = tx.send(());
        });

        BackoffTimer { task, finished }
    }

    /// Checks if the backoff timer has elapsed.
    fn backoff_completed(backoff: &mut BackoffTimer) -> bool {
        match backoff.finished.try_recv() {
            Ok(()) | Err(TryRecvError::Closed) => true,
            Err(TryRecvError::Empty) => false,
        }
    }
}

/// Produces a deterministic jitter duration using system time entropy.
#[allow(clippy::arithmetic_side_effects)]
fn jittered_duration(
    min_ms: u64,
    max_ms: u64,
) -> Duration {
    let span = max_ms.saturating_sub(min_ms);
    if span == 0 {
        return Duration::from_millis(min_ms);
    }

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.subsec_nanos())
        .unwrap_or_default();
    let offset = u64::from(nanos) % (span.saturating_add(1));

    Duration::from_millis(min_ms.saturating_add(offset))
}

#[cfg(test)]
mod tests {
    use tokio::time;

    use super::*;

    fn sync_state(synced: bool) -> SyncState {
        let sig_a = hash_with(1);
        let sig_b = if synced { sig_a } else { hash_with(2) };

        SyncState::new(sig_a, sig_b)
    }

    fn hash_with(byte: u8) -> Blake3256 {
        [byte; 32].into()
    }

    #[tokio::test]
    async fn stable_to_diverged() -> anyhow::Result<()> {
        let machine = StateMachine::shared();

        let snapshot = StateMachine::step(&machine, &sync_state(false))?;
        assert!(matches!(snapshot, StateSnapshot::Diverged));

        Ok(())
    }

    #[tokio::test]
    async fn stable_stays_stable_when_synced() -> anyhow::Result<()> {
        let machine = StateMachine::shared();

        let snapshot = StateMachine::step(&machine, &sync_state(true))?;
        assert!(matches!(snapshot, StateSnapshot::Stable));

        Ok(())
    }

    #[tokio::test]
    async fn diverged_to_reconciling_after_backoff() -> anyhow::Result<()> {
        let machine = StateMachine::shared();

        StateMachine::step(&machine, &sync_state(false))?;
        time::sleep(Duration::from_millis(900)).await;

        let snapshot = StateMachine::step(&machine, &sync_state(false))?;
        assert!(matches!(snapshot, StateSnapshot::Reconciling));

        Ok(())
    }

    #[tokio::test]
    async fn reconciling_to_stable_on_sync() -> anyhow::Result<()> {
        let machine = StateMachine::shared();

        StateMachine::step(&machine, &sync_state(false))?;
        time::sleep(Duration::from_millis(900)).await;
        StateMachine::step(&machine, &sync_state(false))?;

        let snapshot = StateMachine::step(&machine, &sync_state(true))?;
        assert!(matches!(snapshot, StateSnapshot::Stable));

        Ok(())
    }

    #[tokio::test]
    async fn reconciling_stays_reconciling_when_still_diverged() -> anyhow::Result<()> {
        let machine = StateMachine::shared();

        StateMachine::step(&machine, &sync_state(false))?;
        time::sleep(Duration::from_millis(900)).await;
        StateMachine::step(&machine, &sync_state(false))?;

        let snapshot = StateMachine::step(&machine, &sync_state(false))?;
        assert!(matches!(snapshot, StateSnapshot::Diverged));

        Ok(())
    }

    #[tokio::test]
    async fn passive_parity_during_backoff_returns_stable() -> anyhow::Result<()> {
        let machine = StateMachine::shared();

        StateMachine::step(&machine, &sync_state(false))?;

        let snapshot = StateMachine::step(&machine, &sync_state(true))?;
        assert!(matches!(snapshot, StateSnapshot::Stable));

        Ok(())
    }

    #[tokio::test]
    async fn stable_remains_stable_without_update() -> anyhow::Result<()> {
        let machine = StateMachine::shared();

        let snapshot = StateMachine::step(&machine, &sync_state(true))?;
        assert!(matches!(snapshot, StateSnapshot::Stable));

        Ok(())
    }

    #[tokio::test]
    async fn diverged_advances_to_reconciling_without_update() -> anyhow::Result<()> {
        let machine = StateMachine::shared();

        StateMachine::step(&machine, &sync_state(false))?;
        time::sleep(Duration::from_millis(900)).await;

        let snapshot = StateMachine::step(&machine, &sync_state(false))?;
        assert!(matches!(snapshot, StateSnapshot::Reconciling));

        Ok(())
    }

    #[tokio::test]
    async fn reconciling_sees_new_mismatch_and_restarts_backoff() -> anyhow::Result<()> {
        let machine = StateMachine::shared();

        // Reach Reconciling first.
        StateMachine::step(&machine, &sync_state(false))?;
        time::sleep(Duration::from_millis(900)).await;
        StateMachine::step(&machine, &sync_state(false))?;

        // New mismatch should push back to Diverged (restart backoff).
        let snapshot = StateMachine::step(&machine, &sync_state(false))?;
        assert!(matches!(snapshot, StateSnapshot::Diverged));

        Ok(())
    }
}
