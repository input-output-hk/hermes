//! Monotonic clock host implementation for WASM runtime.

use super::state::monotonic_clock_now;
use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::wasi::clocks::monotonic_clock::{Duration, Host, Instant},
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
        let res_duration = std::time::Duration::from_nanos(1);
        Ok(res_duration.as_nanos().try_into()?)
    }
}
