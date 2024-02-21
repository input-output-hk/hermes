//! Localtime runtime extension implementation.

use time::{Date, Duration, Month, OffsetDateTime, Time};
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
/// * `when`: `Option<Datetime>`.
/// * `tz`: `Option<Timezone>`.
///
/// **Returns**
/// `wasmtime::Result<Result<Localtime, Errno>>`.
fn get_localtime_impl(
    when: Option<Datetime>, tz: Option<Timezone>,
) -> wasmtime::Result<Result<Localtime, Errno>> {
    let date_time = get_dt(when)?;
    let timezone = get_tz(tz)?;
    let dt = date_time.to_timezone(timezone);
    let localtime = offset_datetime_to_localtime(dt, timezone.name());
    Ok(localtime)
}

/// Get alt localtime from a datetime or now.
///
/// **Parameters**
/// * `time`: `Localtime`.
/// * `tz`: `Option<Timezone>`.
///
/// **Returns**
/// `wasmtime::Result<Result<Localtime, Errno>>`.
fn alt_localtime_impl(
    time: Localtime, tz: Option<Timezone>,
) -> wasmtime::Result<Result<Localtime, Errno>> {
    let alt_tz = get_tz(tz)?;
    let date_time: OffsetDateTime = time.try_into()?;
    let dt = date_time.to_timezone(alt_tz);
    let alt_time = offset_datetime_to_localtime(dt, alt_tz.name());
    Ok(alt_time)
}

/// Get datetime from a localtime.
///
/// **Parameters**
/// * `time`: `Localtime`.
///
/// **Returns**
/// `wasmtime::Result<Result<Datetime, Errno>>`.
fn get_datetime_impl(time: Localtime) -> wasmtime::Result<Result<Datetime, Errno>> {
    let dt: OffsetDateTime = time.try_into()?;
    Ok(dt.try_into())
}

/// Get `OffsetDateTime` from a `Datetime` or now.
///
/// **Parameters**
/// * `when`: `Option<Datetime>`.
///
/// **Returns**
/// `wasmtime::Result<Result<OffsetDateTime, Errno>>`.
fn get_dt(dt: Option<Datetime>) -> Result<OffsetDateTime, Errno> {
    let dt = match dt {
        Some(Datetime {
            seconds,
            nanoseconds,
        }) => {
            let seconds = seconds.try_into().map_err(|_| Errno::InvalidLocaltime)?;
            OffsetDateTime::from_unix_timestamp(seconds).map_err(|_| Errno::InvalidLocaltime)?
                + Duration::nanoseconds(nanoseconds.into())
        },
        None => OffsetDateTime::now_local().map_err(|_| Errno::InvalidLocaltime)?,
    };
    Ok(dt)
}

/// Get `Tz` from a `Timezone`.
///
/// **Parameters**
/// * `tz`: `Option<Timezone>`.
///
/// **Returns**
/// `wasmtime::Result<Result<Tz, Errno>>`.
fn get_tz<'a>(tz: Option<Timezone>) -> Result<&'a Tz, Errno> {
    let timezone = if let Some(tz_str) = tz {
        get_tz_by_name(&tz_str)?
    } else {
        get_timezone().map_err(|_| Errno::UnknownTimezone)?
    };
    Ok(timezone)
}

/// Get `Tz` by name.
///
/// **Parameters**
/// * `tz_str`: `&str`.
///
/// **Returns**
/// `wasmtime::Result<Result<Tz, Errno>>`.
fn get_tz_by_name<'a>(tz_str: &str) -> Result<&'a Tz, Errno> {
    timezones::get_by_name(tz_str).ok_or(Errno::UnknownTimezone)
}

/// Convert `OffsetDateTime` and timezone name,  to `Localtime`.
///
/// **Parameters**
/// * `dt`: `OffsetDateTime`.
/// * `tz_name`: `&str`.
///
/// **Returns**
/// `wasmtime::Result<Result<Localtime, Errno>>`.
fn offset_datetime_to_localtime(dt: OffsetDateTime, tz_name: &str) -> Result<Localtime, Errno> {
    let localtime = Localtime {
        year: dt.year().try_into().map_err(|_| Errno::YearOutOfRange)?,
        month: dt.month() as u8,
        dow: dt.sunday_based_week(),
        day: dt.day(),
        hh: dt.hour(),
        mm: dt.minute(),
        ss: dt.second(),
        ns: dt.nanosecond(),
        tz: tz_name.to_string(),
    };
    Ok(localtime)
}

impl TryInto<OffsetDateTime> for Localtime {
    type Error = Errno;

    fn try_into(self) -> Result<OffsetDateTime, Self::Error> {
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
        } = self;
        let orig_tz = get_tz(Some(orig_tz))?;
        let offset = orig_tz.get_offset_primary().to_utc();
        let year = year.try_into().map_err(|_| Errno::YearOutOfRange)?;
        let month: Month = month.try_into().map_err(|_| Errno::InvalidLocaltime)?;
        let date =
            Date::from_calendar_date(year, month, day).map_err(|_| Errno::InvalidLocaltime)?;
        let time = Time::from_hms_nano(hh, mm, ss, ns).map_err(|_| Errno::InvalidLocaltime)?;
        let date_time = OffsetDateTime::new_in_offset(date, time, offset);
        Ok(date_time)
    }
}

impl TryFrom<OffsetDateTime> for Datetime {
    type Error = Errno;

    fn try_from(value: OffsetDateTime) -> Result<Self, Self::Error> {
        let seconds = value
            .unix_timestamp()
            .try_into()
            .map_err(|_| Errno::InvalidLocaltime)?;
        let nanoseconds = value.nanosecond();
        Ok(Datetime {
            seconds,
            nanoseconds,
        })
    }
}
