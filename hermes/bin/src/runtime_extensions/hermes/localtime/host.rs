//! Localtime host implementation for WASM runtime.

use chrono::{Local, TimeZone};
use chrono_tz::Tz;

use crate::{
    runtime_extensions::{
        bindings::{
            hermes::localtime::api::{Errno, Host, Localtime, Timezone},
            wasi::clocks::wall_clock::Datetime,
        },
        hermes::localtime::get_tz,
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
        let timezone = get_tz(tz)?;
        let local_naive = match when {
            Some(Datetime {
                seconds,
                nanoseconds,
            }) => {
                let seconds = seconds.try_into().map_err(|_| Errno::InvalidLocaltime)?;
                let utc_dt = chrono::DateTime::from_timestamp(seconds, nanoseconds)
                    .ok_or(Errno::InvalidLocaltime)?;
                utc_dt.naive_utc()
            },
            None => Local::now().naive_utc(),
        };
        let local_date_time = timezone.from_utc_datetime(&local_naive);
        Ok(local_date_time.try_into())
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
        let local_date_time: chrono::DateTime<Tz> = time.try_into()?;
        let alt_local_date_time = match tz {
            Some(alt_tz) => {
                let tz = get_tz(Some(alt_tz))?;
                tz.from_utc_datetime(&local_date_time.naive_utc())
            },
            None => local_date_time,
        };
        Ok(alt_local_date_time.try_into())
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
        Ok(time.try_into())
    }
}
