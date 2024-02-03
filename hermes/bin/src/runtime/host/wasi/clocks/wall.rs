//! Host - WASI - Wall Clock implementations

use crate::runtime::extensions::{
    wasi::clocks::wall_clock::{Datetime, Host},
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
    #[doc = " This clock is not monotonic, therefore calling this function repeatedly"]
    #[doc = " will not necessarily produce a sequence of non-decreasing values."]
    #[doc = " "]
    #[doc = " The returned timestamps represent the number of seconds since"]
    #[doc = " 1970-01-01T00:00:00Z, also known as [POSIX\\'s Seconds Since the Epoch],"]
    #[doc = " also known as [Unix Time]."]
    #[doc = " "]
    #[doc = " The nanoseconds field of the output is always less than 1000000000."]
    #[doc = " "]
    #[doc = " [POSIX\\'s Seconds Since the Epoch]: https://pubs.opengroup.org/onlinepubs/9699919799/xrat/V4_xbd_chap04.html#tag_21_04_16"]
    #[doc = " [Unix Time]: https://en.wikipedia.org/wiki/Unix_time"]
    fn now(&mut self) -> wasmtime::Result<Datetime> {
        todo!()
    }

    #[doc = " Query the resolution of the clock."]
    #[doc = " "]
    #[doc = " The nanoseconds field of the output is always less than 1000000000."]
    fn resolution(&mut self) -> wasmtime::Result<Datetime> {
        todo!()
    }
}
