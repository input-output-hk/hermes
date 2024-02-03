//! Host - Localtime implementations
//!
#![allow(unused_variables)]

use crate::runtime::extensions::{
    hermes::localtime::api::{Errno, Host, Localtime, Timezone},
    wasi::clocks::wall_clock::Datetime,
};

/// State
struct State {}

impl Host for State {
    #[doc = " Get localtime from a datetime or now."]
    #[doc = " "]
    #[doc = " **Parameters**"]
    #[doc = " "]
    #[doc = " `when` : The datetime we want to convert (Optional, if not set it will convert `now`)."]
    #[doc = " `tz` : The timezone to use. (Optional, if not set uses the local machines configured local timezone.)"]
    #[doc = " "]
    #[doc = " **Returns**"]
    #[doc = " "]
    #[doc = " `localtime` : the converted time."]
    #[doc = " `errno`     : An error indicating why conversion failed."]
    fn get_localtime(
        &mut self, when: Option<Datetime>, tz: Option<Timezone>,
    ) -> wasmtime::Result<Result<Localtime, Errno>> {
        todo!()
    }

    #[doc = " Get a new localtime from a localtime, by recalculating time for a new timezone."]
    #[doc = " "]
    #[doc = " **Parameters**"]
    #[doc = " "]
    #[doc = " `time` : The localtime to convert."]
    #[doc = " `tz` : The timezone to use. (Optional, if not set uses the local machines configured local timezone.)"]
    #[doc = " "]
    #[doc = " **Returns**"]
    #[doc = " "]
    #[doc = " `localtime` : the converted time."]
    #[doc = " `errno`     : An error indicating why conversion failed."]
    fn alt_localtime(
        &mut self, time: Localtime, tz: Option<Timezone>,
    ) -> wasmtime::Result<Result<Localtime, Errno>> {
        todo!()
    }

    #[doc = " Get a datetime from a localtime."]
    #[doc = " "]
    #[doc = " **Parameters**"]
    #[doc = " "]
    #[doc = " `time` : The localtime to convert."]
    #[doc = " "]
    #[doc = " **Returns**"]
    #[doc = " "]
    #[doc = " `datetime`  : the converted time."]
    #[doc = " `errno`     : An error indicating why conversion failed."]
    fn get_datetime(&mut self, time: Localtime) -> wasmtime::Result<Result<Datetime, Errno>> {
        todo!()
    }
}
