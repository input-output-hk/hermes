//! Implementation of exit status retrieval after event queue shutdown.

use std::{
    process::ExitCode,
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

/// Exit status.
#[derive(Copy, Clone, Debug, thiserror::Error)]
pub enum Exit {
    /// An application requested runtime abort.
    #[error(
        "Event queue closed: an application requested runtime abort \
         with exit code ({exit_code:?})"
    )]
    Done {
        /// Exit code provided by an application.
        exit_code: ExitCode,
    },
    /// All event senders are dropped.
    #[error("Event queue closed: all event senders are dropped")]
    QueueClosed,
    /// Another task panicked inside.
    #[error("Event queue poisoned: another task panicked inside")]
    QueuePoisoned,
    /// Timeout elapsed.
    #[error("Event queue closed: timeout")]
    Timeout,
}

impl Exit {
    /// Returns an exit code if `Self` is [`Self::Done`].
    pub fn get_exit_code(self) -> Option<ExitCode> {
        if let Self::Done { exit_code } = self {
            Some(exit_code)
        } else {
            None
        }
    }

    /// Returns either an exit code from [`Self::Done`] or [`ExitCode::FAILURE`].
    pub fn unwrap_exit_code_or_failure(self) -> ExitCode {
        self.get_exit_code().unwrap_or(ExitCode::FAILURE)
    }
}

impl From<ExitCode> for Exit {
    fn from(exit_code: ExitCode) -> Self {
        Self::Done { exit_code }
    }
}

/// Lock to [`Exit`] that event queue sets on shutdown.
// To satisfy the guarantees [`ExitLock`] should never implement or derive `Clone`.
pub struct ExitLock(Arc<(Condvar, Mutex<Option<Exit>>)>);

impl ExitLock {
    /// Initializes the lock and return two clones of it.
    pub(super) fn new_pair() -> (Self, Self) {
        let condvar = Condvar::new();
        // Default value doesn't matter as it's bound to be awaited to be changed.
        let payload = Mutex::new(None);
        let inner = Arc::new((condvar, payload));
        (Self(inner.clone()), Self(inner))
    }

    /// Set the [`Exit`] value. This will notify the waiting thread.
    ///
    /// This shouldn't exposed the waiting thread.
    pub(super) fn set(self, exit: Exit) {
        let (condvar, payload) = &*self.0;
        // It doesn't matter if the lock is poisoned
        // because the condvar would catch it anyway.
        let Ok(mut payload) = payload.lock() else {
            return;
        };
        payload.get_or_insert(exit);
        condvar.notify_one();
    }

    /// Blocks until the [`Exit`] value is set.
    pub fn wait(self) -> Exit {
        let (condvar, payload) = &*self.0;
        let Ok(exit) = payload.lock() else {
            return Exit::QueuePoisoned;
        };
        condvar
            .wait_while(exit, |opt| opt.is_none())
            .as_deref()
            .copied()
            .unwrap_or(Some(Exit::QueuePoisoned))
            // `None` is guaranteed to be unreachable since we are waiting for `Some` on condvar.
            .unwrap_or(Exit::QueuePoisoned)
    }

    /// Blocks until either the [`Exit`] value is set or the timeout elapses.
    pub fn wait_timeout(self, dur: Duration) -> Exit {
        let (condvar, payload) = &*self.0;
        let Ok(exit) = payload.lock() else {
            return Exit::QueuePoisoned;
        };
        condvar
            .wait_timeout_while(exit, dur, |opt| opt.is_none())
            .map(|(exit, timeout)| {
                if timeout.timed_out() {
                    Some(Exit::Timeout)
                } else {
                    *exit
                }
            })
            .unwrap_or(Some(Exit::QueuePoisoned))
            // `None` is guaranteed to be unreachable since we are waiting for `Some` on condvar.
            .unwrap_or(Exit::QueuePoisoned)
    }
}
