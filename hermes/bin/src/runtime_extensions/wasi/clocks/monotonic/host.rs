//! Monotonic clock host implementation for WASM runtime.

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::wasi::clocks::monotonic_clock::{Duration, Host, Instant, Pollable},
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

    ///  Create a `pollable` which will resolve once the specified instant
    ///  has occurred.
    fn subscribe_instant(
        &mut self,
        _when: Instant,
    ) -> wasmtime::Result<wasmtime::component::Resource<Pollable>> {
        todo!()
    }

    /// Create a `pollable` that will resolve after the specified duration has
    /// elapsed from the time this function is invoked."]
    fn subscribe_duration(
        &mut self,
        _when: Duration,
    ) -> wasmtime::Result<wasmtime::component::Resource<Pollable>> {
        todo!()
    }
}
