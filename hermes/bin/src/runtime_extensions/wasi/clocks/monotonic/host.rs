//! Monotonic clock host implementation for WASM runtime.

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::wasi::clocks::monotonic_clock::{Duration, Host, Instant},
        wasi::clocks::state::{monotonic_clock_now, monotonic_clock_res},
    },
};

impl Host for HermesRuntimeContext {
    /// Read the current value of the clock.
    ///
    /// The clock is monotonic, therefore calling this function repeatedly will
    /// produce a sequence of non-decreasing values.
    fn now(&mut self) -> wasmtime::Result<Instant> {
        monotonic_clock_now()
    }

    /// Query the resolution of the clock. Returns the duration of time
    /// corresponding to a clock tick.
    fn resolution(&mut self) -> wasmtime::Result<Duration> {
        Ok(monotonic_clock_res())
    }
}
