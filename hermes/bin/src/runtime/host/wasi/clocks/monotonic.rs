//! Host - WASI - monotonic clock implementations

use crate::{
    runtime::extensions::{
        bindings::wasi::clocks::monotonic_clock::{Duration, Host, Instant},
        state::{Context, Stateful},
    },
    state::HermesState,
};

/// WASI State
pub(crate) struct State {}

impl Stateful for State {
    fn new(_ctx: &Context) -> Self {
        Self {}
    }
}

impl Host for HermesState {
    /// Read the current value of the clock.
    ///
    /// The clock is monotonic, therefore calling this function repeatedly will
    /// produce a sequence of non-decreasing values.
    fn now(&mut self) -> wasmtime::Result<Instant> {
        todo!()
    }

    /// Query the resolution of the clock. Returns the duration of time
    /// corresponding to a clock tick.
    fn resolution(&mut self) -> wasmtime::Result<Duration> {
        todo!()
    }
}
