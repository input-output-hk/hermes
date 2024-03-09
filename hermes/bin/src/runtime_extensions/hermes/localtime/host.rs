//! Localtime host implementation for WASM runtime.

use crate::{
    runtime_extensions::{
        bindings::{
            hermes::localtime::api::{Errno, Host, Localtime, Timezone},
            wasi::clocks::wall_clock::Datetime,
        },
        hermes::localtime::{alt_localtime_impl, get_datetime_impl, get_localtime_impl},
    },
    state::HermesState,
};

impl Host for HermesState {
    /// Get localtime from a datetime or now.
    ///
    /// **Parameters**
    ///
    /// `when` : The datetime we want to convert (Optional, if not set it will convert
    /// `now`).
    /// `tz` : The timezone to use. (Optional, if not set uses the local machines
    /// configured local timezone.)
    ///
    /// **Returns**
    ///
    /// `localtime` : the converted time.
    /// `errno`     : An error indicating why conversion failed.
    fn get_localtime(
        &mut self, when: Option<Datetime>, tz: Option<Timezone>,
    ) -> wasmtime::Result<Result<Localtime, Errno>> {
        get_localtime_impl(when, tz)
    }

    /// Get a new localtime from a localtime, by recalculating time for a new timezone.
    ///
    /// **Parameters**
    ///
    /// `time` : The localtime to convert.
    /// `tz` : The timezone to use. (Optional, if not set uses the local machines
    /// configured local timezone.)
    ///
    /// **Returns**
    ///
    /// `localtime` : the converted time.
    /// `errno`     : An error indicating why conversion failed.
    fn alt_localtime(
        &mut self, time: Localtime, tz: Option<Timezone>,
    ) -> wasmtime::Result<Result<Localtime, Errno>> {
        alt_localtime_impl(time, tz)
    }

    /// Get a datetime from a localtime.
    ///
    /// **Parameters**
    ///
    /// `time` : The localtime to convert.
    ///
    /// **Returns**
    ///
    /// `datetime`  : the converted time.
    /// `errno`     : An error indicating why conversion failed.
    fn get_datetime(&mut self, time: Localtime) -> wasmtime::Result<Result<Datetime, Errno>> {
        get_datetime_impl(time)
    }
}
