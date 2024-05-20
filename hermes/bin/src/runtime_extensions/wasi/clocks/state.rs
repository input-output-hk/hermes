//! Clock State.

use std::time::SystemTime;

use once_cell::sync::Lazy;

use crate::runtime_extensions::bindings::wasi::clocks::{
    monotonic_clock::Instant, wall_clock::Datetime,
};

/// Clock state singleton.
static CLOCK_STATE: Lazy<ClockState> = Lazy::new(ClockState::new);

/// Clock state.
struct ClockState {
    /// Monotonic clock base instant.
    base: std::time::Instant,
    /// Monotonic clock resolution.
    mono_resolution: Instant,
    /// Wall clock resolution.
    wall_resolution: Datetime,
}

/// Clock state implementation.
impl ClockState {
    /// Creates a new instance of the `ClockState`.
    fn new() -> Self {
        // This should not fail, in case it does, log error and return the value of 1
        // nanosecond.
        let res_duration = std::time::Duration::from_nanos(1);
        let mono_resolution = match res_duration.as_nanos().try_into() {
            Ok(res) => res,
            Err(res_err) => {
                tracing::error!(message = "Error converting duration to nanoseconds!", error = %res_err);
                1
            },
        };
        let wall_resolution = Datetime {
            seconds: res_duration.as_secs(),
            nanoseconds: res_duration.subsec_nanos(),
        };
        Self {
            base: std::time::Instant::now(),
            mono_resolution,
            wall_resolution,
        }
    }

    /// Returns the current value of the monotonic clock.
    fn monotonic_now(&self) -> wasmtime::Result<Instant> {
        Ok(Instant::try_from(self.base.elapsed().as_nanos())?)
    }
}

/// Monotonic Clock current time.
pub(crate) fn monotonic_clock_now() -> wasmtime::Result<Instant> {
    CLOCK_STATE.monotonic_now()
}

/// Monotonic Clock resolution.
pub(crate) fn monotonic_clock_res() -> Instant {
    CLOCK_STATE.mono_resolution
}

/// Wall Clock current time.
pub(crate) fn wall_clock_now() -> wasmtime::Result<Datetime> {
    Ok(SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| Datetime {
            seconds: d.as_secs(),
            nanoseconds: d.subsec_nanos(),
        })?)
}

/// Wall Clock resolution.
pub(crate) fn wall_clock_res() -> Datetime {
    CLOCK_STATE.wall_resolution
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_monotonic_clock_now() {
        let one = monotonic_clock_now().unwrap();
        let two = monotonic_clock_now().unwrap();
        assert!(one <= two);
    }

    #[test]
    fn test_monotonic_clock_state() {
        let one = CLOCK_STATE.monotonic_now().unwrap();
        let two = CLOCK_STATE.monotonic_now().unwrap();
        assert!(one <= two);
    }

    #[test]
    fn test_threaded_monotonic_clock_state() {
        let handle_two = std::thread::spawn(|| {
            (
                CLOCK_STATE.monotonic_now().unwrap(),
                CLOCK_STATE.monotonic_now().unwrap(),
            )
        });
        let handle_one = std::thread::spawn(|| {
            (
                CLOCK_STATE.monotonic_now().unwrap(),
                CLOCK_STATE.monotonic_now().unwrap(),
            )
        });
        let (one, two) = handle_one.join().unwrap();
        let (three, four) = handle_two.join().unwrap();
        assert!(one <= two);
        assert!(three <= four);
    }
}
