//! Localtime host implementation for WASM runtime.

use chrono::{Local, TimeZone};
use chrono_tz::Tz;

use crate::runtime_extensions::{
    bindings::{
        hermes::localtime::api::{Errno, Localtime, Timezone},
        wasi::clocks::wall_clock::Datetime,
    },
    hermes::localtime::get_tz,
};

/// (Implementation) Get localtime from a datetime or now.
pub(super) fn get_localtime(
    when: Option<Datetime>,
    tz: Option<Timezone>,
) -> Result<Localtime, Errno> {
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

    local_date_time.try_into()
}

/// (Implementation) Get a new localtime from a localtime, by recalculating time for a new
/// timezone.
pub(super) fn alt_localtime(
    time: Localtime,
    tz: Option<Timezone>,
) -> Result<Localtime, Errno> {
    let local_date_time: chrono::DateTime<Tz> = time.try_into()?;
    let alt_local_date_time = match tz {
        Some(alt_tz) => {
            let tz = get_tz(Some(alt_tz))?;
            tz.from_utc_datetime(&local_date_time.naive_utc())
        },
        None => local_date_time,
    };

    alt_local_date_time.try_into()
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;

    #[test]
    fn test_get_localtime_with_utc_offset() {
        let result = get_localtime(None, Some(String::from("Europe/London")));
        assert!(result.is_ok()); // Check if the function call was successful
    }
}
