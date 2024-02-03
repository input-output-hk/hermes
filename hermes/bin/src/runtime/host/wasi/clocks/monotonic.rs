//! Host - WASI - monotonic clock implementations
//!
#![allow(unused_variables)]

use crate::runtime::extensions::{
    wasi::clocks::monotonic_clock::{Duration, Host, Instant},
    HermesState, NewState,
};

/// WASI State
pub(crate) struct State {}

impl NewState for State {
    fn new(ctx: &crate::wasm::context::Context) -> Self {
        Self {}
    }
}

impl Host for HermesState {
    #[doc = " Read the current value of the clock."]
    #[doc = " "]
    #[doc = " The clock is monotonic, therefore calling this function repeatedly will"]
    #[doc = " produce a sequence of non-decreasing values."]
    fn now(&mut self) -> wasmtime::Result<Instant> {
        todo!()
    }

    #[doc = " Query the resolution of the clock. Returns the duration of time"]
    #[doc = " corresponding to a clock tick."]
    fn resolution(&mut self) -> wasmtime::Result<Duration> {
        todo!()
    }
}
