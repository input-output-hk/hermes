//! Monotonic clock runtime extension implementation.

use libc::{clock_getres, timespec, CLOCK_MONOTONIC};

use crate::runtime_extensions::{
    bindings::wasi::clocks::monotonic_clock::{Duration, Instant},
    state::{Context, Stateful},
};

mod host;

/// WASI State
pub(crate) struct State {}

impl Stateful for State {
    fn new(_ctx: &Context) -> Self {
        Self {}
    }
}

/// Read the current value of the clock.
///
/// The clock is monotonic, therefore calling this function repeatedly will
/// produce a sequence of non-decreasing values.
fn now_impl() -> wasmtime::Result<Instant> {
    todo!()
}

/// Query the resolution of the clock. Returns the duration of time
/// corresponding to a clock tick.
///
/// The resolution of the clock may vary based on the environment.
///
/// Returns an error if the resolution cannot be determined.
fn resolution_impl() -> wasmtime::Result<Duration> {
    // Run the clock_getres syscall to get the resolution of the clock.
    // https://man7.org/linux/man-pages/man2/clock_getres.2.html
    let timespec { tv_sec, tv_nsec } = unsafe {
        let mut tp = timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let clk_id = CLOCK_MONOTONIC;
        clock_getres(clk_id, &mut tp);
        tp
    };
    let seconds: u64 = tv_sec
        .try_into()
        .map_err(|_| ClockError::InvalidClockResolution)?;
    let nanoseconds: u64 = tv_nsec
        .try_into()
        .map_err(|_| ClockError::InvalidClockResolution)?;
    let duration = seconds
        .checked_mul(1_000_000_000)
        .ok_or(ClockError::InvalidClockResolution)?
        .checked_add(nanoseconds)
        .ok_or(ClockError::InvalidClockResolution)?;
    Ok(duration)
}

/// Clock runtime extension implementation errors
#[derive(thiserror::Error, Debug)]
enum ClockError {
    /// Invalid clock resolution error.
    #[error("Invalid clock resolution")]
    InvalidClockResolution,
}
