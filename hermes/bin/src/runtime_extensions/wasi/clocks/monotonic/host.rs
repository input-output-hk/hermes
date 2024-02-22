//! Monotonic clock host implementation for WASM runtime.

use super::{now_impl, resolution_impl};
use crate::{
    runtime_extensions::bindings::wasi::clocks::monotonic_clock::{Duration, Host, Instant},
    state::HermesState,
};

impl Host for HermesState {
    /// Read the current value of the clock.
    ///
    /// The clock is monotonic, therefore calling this function repeatedly will
    /// produce a sequence of non-decreasing values.
    fn now(&mut self) -> wasmtime::Result<Instant> {
        now_impl()
    }

    /// Query the resolution of the clock. Returns the duration of time
    /// corresponding to a clock tick.
    fn resolution(&mut self) -> wasmtime::Result<Duration> {
        resolution_impl()
    }
}
