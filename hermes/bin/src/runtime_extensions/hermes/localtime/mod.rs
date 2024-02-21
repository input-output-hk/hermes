//! Localtime runtime extension implementation.

use time::{Date, Duration, OffsetDateTime, Time};
use time_tz::{system::get_timezone, timezones, Offset, OffsetDateTimeExt, TimeZone, Tz};

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
    when: Option<Datetime>, tz: Option<Timezone>,
) -> wasmtime::Result<Result<Localtime, Errno>> {
    let date_time = if let Some(Datetime {
        seconds,
        nanoseconds,
    }) = when
    {
        OffsetDateTime::from_unix_timestamp(seconds.try_into()?)
            .map_err(|_| Errno::InvalidLocaltime)?
            + Duration::nanoseconds(nanoseconds.into())
    } else {
        OffsetDateTime::now_local()?
    };

    let timezone = get_tz(tz)?;

    let dt = date_time.to_timezone(timezone);

    let localtime = Localtime {
        year: dt.year().try_into().map_err(|_| Errno::YearOutOfRange)?,
        month: dt.month() as u8,
        dow: dt.sunday_based_week(),
        day: dt.day(),
        hh: dt.hour(),
        mm: dt.minute(),
        ss: dt.second(),
        ns: dt.nanosecond(),
        tz: timezone.name().to_string(),
    };
    Ok(Ok(localtime))
}

fn alt_localtime_impl(
    time: Localtime, tz: Option<Timezone>,
) -> wasmtime::Result<Result<Localtime, Errno>> {
    let alt_tz = get_tz(tz)?;
    let Localtime {
        year,
        month,
        dow: _,
        day,
        hh,
        mm,
        ss,
        ns,
        tz: orig_tz,
    } = time;
    let orig_tz = get_tz(Some(orig_tz))?;
    let offset = orig_tz.get_offset_primary().to_utc();
    let date = Date::from_calendar_date(year.try_into()?, month.try_into()?, day.into())?;
    let time = Time::from_hms_nano(hh, mm, ss, ns)?;
    let date_time = OffsetDateTime::new_in_offset(date, time, offset);

    let dt = date_time.to_timezone(alt_tz);

    let alt_time = Localtime {
        year: dt.year().try_into().map_err(|_| Errno::YearOutOfRange)?,
        month: dt.month() as u8,
        dow: dt.sunday_based_week(),
        day: dt.day(),
        hh: dt.hour(),
        mm: dt.minute(),
        ss: dt.second(),
        ns: dt.nanosecond(),
        tz: alt_tz.name().to_string(),
    };
    Ok(Ok(alt_time))
}

fn get_tz<'a>(tz: Option<Timezone>) -> Result<&'a Tz, Errno> {
    let timezone = if let Some(tz_str) = tz {
        get_tz_by_name(&tz_str)?
    } else {
        get_timezone().map_err(|_| Errno::UnknownTimezone)?
    };
    Ok(timezone)
}

fn get_tz_by_name<'a>(tz_str: &str) -> Result<&'a Tz, Errno> {
    timezones::get_by_name(&tz_str).ok_or(Errno::UnknownTimezone)
}
