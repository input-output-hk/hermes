//! Host - Localtime implementations

use crate::{
    runtime::extensions::{
        bindings::hermes::localtime::api::{Errno, Host, Localtime, Timezone},
        bindings::wasi::clocks::wall_clock::Datetime,
    },
    state::{HermesState, Stateful},
};

/// State
pub(crate) struct State {}

impl Stateful for State {
    fn new(_ctx: &crate::state::Context) -> Self {
        State {}
    }
}

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
        &mut self, _when: Option<Datetime>, _tz: Option<Timezone>,
    ) -> wasmtime::Result<Result<Localtime, Errno>> {
        todo!()
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
        &mut self, _time: Localtime, _tz: Option<Timezone>,
    ) -> wasmtime::Result<Result<Localtime, Errno>> {
        todo!()
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
    fn get_datetime(&mut self, _time: Localtime) -> wasmtime::Result<Result<Datetime, Errno>> {
        todo!()
    }
}
