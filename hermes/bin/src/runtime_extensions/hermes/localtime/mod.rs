//! Localtime runtime extension implementation.

use time::{Duration, OffsetDateTime};

use crate::runtime_extensions::{
    bindings::{
        hermes::localtime::api::{Errno, Localtime, Timezone},
        wasi::clocks::wall_clock::Datetime,
    },
    state::{Context, Stateful},
};

mod host;

/// State
pub(crate) struct State {}

impl Stateful for State {
    fn new(_ctx: &Context) -> Self {
        State {}
    }
}

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
fn get_localtime_impl(
    when: Option<Datetime>, _tz: Option<Timezone>,
) -> wasmtime::Result<Result<Localtime, Errno>> {
    let _datetime = match when {
        Some(Datetime {
            seconds,
            nanoseconds,
        }) => {
            let datetime = OffsetDateTime::from_unix_timestamp(seconds.try_into()?)?
                + Duration::nanoseconds(nanoseconds.try_into()?);
        },
        None => todo!(),
    };
    todo!()
}
