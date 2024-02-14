//! Host - WASI - Wall Clock implementations

use crate::{
    runtime::extensions::{
        bindings::wasi::clocks::wall_clock::{Datetime, Host},
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
    /// This clock is not monotonic, therefore calling this function repeatedly
    /// will not necessarily produce a sequence of non-decreasing values.
    ///
    /// The returned timestamps represent the number of seconds since
    /// 1970-01-01T00:00:00Z, also known as [POSIX\'s Seconds Since the Epoch],
    /// also known as [Unix Time].
    ///
    /// The nanoseconds field of the output is always less than 1000000000.
    ///
    /// [POSIX\'s Seconds Since the Epoch]: https://pubs.opengroup.org/onlinepubs/9699919799/xrat/V4_xbd_chap04.html#tag_21_04_16
    /// [Unix Time]: https://en.wikipedia.org/wiki/Unix_time
    fn now(&mut self) -> wasmtime::Result<Datetime> {
        todo!()
    }

    /// Query the resolution of the clock.
    ///
    /// The nanoseconds field of the output is always less than 1000000000.
    fn resolution(&mut self) -> wasmtime::Result<Datetime> {
        todo!()
    }
}
