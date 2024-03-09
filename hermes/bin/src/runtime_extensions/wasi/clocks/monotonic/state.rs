//! Monotonic Clock State.

use std::sync::atomic::{AtomicU64, Ordering};

use once_cell::sync::Lazy;

use crate::runtime_extensions::bindings::wasi::clocks::monotonic_clock::Instant;

/// Monotonic clock state singleton.
static MONOTONIC_CLOCK_STATE: Lazy<MonotonicClockState> = Lazy::new(MonotonicClockState::new);

/// Monotonic clock state.
struct MonotonicClockState {
    /// Monotonic clock base instant.
    ///
    /// Every time `now` is called, the duration since `base` is added to the
    /// monotonic clock's `now` value.
    base: std::time::Instant,
    /// Monotonic clock `now` value in nanoseconds.
    now: AtomicU64,
}

/// Monotonic clock state implementation.
impl MonotonicClockState {
    /// Creates a new instance of the `MonotonicClockState`.
    fn new() -> Self {
        Self {
            base: std::time::Instant::now(),
            now: AtomicU64::new(0),
        }
    }

    /// Returns the current value of the monotonic clock.
    fn now(&self) -> wasmtime::Result<Instant> {
        let instant = u64::try_from(self.base.elapsed().as_nanos())?;
        self.now.store(instant, Ordering::SeqCst);
        Ok(self.now.load(Ordering::SeqCst))
    }
}

/// Monotonic clock state now.
pub(crate) fn monotonic_clock_now() -> wasmtime::Result<Instant> {
    MONOTONIC_CLOCK_STATE.now()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_monotonic_clock_now() {
        let one = monotonic_clock_now().unwrap();
        let two = monotonic_clock_now().unwrap();
        assert!(one < two);
    }

    #[test]
    fn test_monotonic_clock_state() {
        let one = MONOTONIC_CLOCK_STATE.now().unwrap();
        let two = MONOTONIC_CLOCK_STATE.now().unwrap();
        assert!(one < two);
    }

    #[test]
    fn test_threaded_monotonic_clock_state() {
        let handle_two = std::thread::spawn(|| {
            (
                MONOTONIC_CLOCK_STATE.now().unwrap(),
                MONOTONIC_CLOCK_STATE.now().unwrap(),
            )
        });
        let handle_one = std::thread::spawn(|| {
            (
                MONOTONIC_CLOCK_STATE.now().unwrap(),
                MONOTONIC_CLOCK_STATE.now().unwrap(),
            )
        });
        let (one, two) = handle_one.join().unwrap();
        println!("one: {one}, two: {two}");
        let (three, four) = handle_two.join().unwrap();
        println!("three: {three}, four: {four}");
        assert!(one < two);
        assert!(three < four);
    }
}
