//! Wall clock host implementation for WASM runtime.

use std::time::SystemTime;

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::wasi::clocks::wall_clock::{Datetime, Host},
};

impl Host for HermesRuntimeContext {
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
        Ok(SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| {
                Datetime {
                    seconds: d.as_secs(),
                    nanoseconds: d.subsec_nanos(),
                }
            })?)
    }

    /// Query the resolution of the clock.
    ///
    /// The nanoseconds field of the output is always less than 1000000000.
    fn resolution(&mut self) -> wasmtime::Result<Datetime> {
        let res_duration = std::time::Duration::from_nanos(1);
        Ok(Datetime {
            seconds: res_duration.as_secs(),
            nanoseconds: res_duration.subsec_nanos(),
        })
    }
}
