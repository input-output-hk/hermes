//! Localtime runtime extension implementation.

use chrono::{Datelike, Local, LocalResult, NaiveDate, TimeZone, Timelike};
use chrono_tz::Tz;

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
pub(crate) fn get_localtime_impl(
    when: Option<Datetime>, tz: Option<Timezone>,
) -> wasmtime::Result<Result<Localtime, Errno>> {
    let date_time = get_dt(when)?;
    let timezone = get_tz(tz)?;
    let dt = date_time.with_timezone(&timezone);
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
pub(crate) fn alt_localtime_impl(
    time: Localtime, tz: Option<Timezone>,
) -> wasmtime::Result<Result<Localtime, Errno>> {
    let alt_tz = get_tz(tz)?;
    let date_time: chrono::DateTime<Tz> = time.try_into()?;
    let dt = date_time.with_timezone(&alt_tz);
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
pub(crate) fn get_datetime_impl(time: Localtime) -> wasmtime::Result<Result<Datetime, Errno>> {
    let dt: chrono::DateTime<Tz> = time.try_into()?;
    Ok(dt.try_into())
}

/// Get `chrono::DateTime<Utc>` from a `Datetime` or now.
///
/// **Parameters**
/// * `when`: `Option<Datetime>`.
///
/// **Returns**
/// `wasmtime::Result<Result<chrono::DateTime<Utc>, Errno>>`.
fn get_dt(dt: Option<Datetime>) -> Result<chrono::DateTime<Tz>, Errno> {
    match dt {
        Some(Datetime {
            seconds,
            nanoseconds,
        }) => {
            let seconds = seconds.try_into().map_err(|_| Errno::InvalidLocaltime)?;
            let utc_dt = chrono::DateTime::from_timestamp(seconds, nanoseconds)
                .ok_or(Errno::InvalidLocaltime)?;
            Ok(Tz::UTC.from_utc_datetime(&utc_dt.naive_utc()))
        },
        None => {
            if let LocalResult::Single(dt) =
                Tz::UTC.from_local_datetime(&Local::now().naive_local())
            {
                Ok(dt)
            } else {
                Err(Errno::InvalidLocaltime)
            }
        },
    }
}

/// Get `Tz` from a `Timezone`.
///
/// **Parameters**
/// * `tz`: `Option<Timezone>`.
///
/// **Returns**
/// `wasmtime::Result<Result<Tz, Errno>>`.
fn get_tz(tz: Option<Timezone>) -> Result<Tz, Errno> {
    let timezone = if let Some(tz_str) = tz {
        tz_str.parse().map_err(|_| Errno::UnknownTimezone)?
    } else {
        let dt = Tz::UTC.from_utc_datetime(&Local::now().naive_local());
        dt.timezone()
    };
    Ok(timezone)
}

/// Convert `chrono::DateTime<Utc>` and timezone name,  to `Localtime`.
///
/// **Parameters**
/// * `dt`: `chrono::DateTime<Utc>`.
/// * `tz_name`: `&str`.
///
/// **Returns**
/// `wasmtime::Result<Result<Localtime, Errno>>`.
fn offset_datetime_to_localtime(
    dt: chrono::DateTime<Tz>, tz_name: &str,
) -> Result<Localtime, Errno> {
    let localtime = Localtime {
        year: dt.year().try_into().map_err(|_| Errno::YearOutOfRange)?,
        month: dt.month().try_into().map_err(|_| Errno::InvalidLocaltime)?,
        dow: dt
            .weekday()
            .number_from_monday()
            .try_into()
            .map_err(|_| Errno::YearOutOfRange)?,
        day: dt.day().try_into().map_err(|_| Errno::InvalidLocaltime)?,
        hh: dt.hour().try_into().map_err(|_| Errno::InvalidLocaltime)?,
        mm: dt
            .minute()
            .try_into()
            .map_err(|_| Errno::InvalidLocaltime)?,
        ss: dt
            .second()
            .try_into()
            .map_err(|_| Errno::InvalidLocaltime)?,
        ns: dt.nanosecond(),
        tz: tz_name.to_string(),
    };
    Ok(localtime)
}

impl TryInto<chrono::DateTime<Tz>> for Localtime {
    type Error = Errno;

    fn try_into(self) -> Result<chrono::DateTime<Tz>, Self::Error> {
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
        let year: i32 = year.try_into().map_err(|_| Errno::YearOutOfRange)?;
        let month: u32 = month.into();
        let day: u32 = day.into();
        let hh: u32 = hh.into();
        let mm: u32 = mm.into();
        let ss: u32 = ss.into();
        let ns: u32 = ns;
        let naive_date_time = NaiveDate::from_ymd_opt(year, month, day)
            .ok_or(Errno::InvalidLocaltime)?
            .and_hms_nano_opt(hh, mm, ss, ns)
            .ok_or(Errno::InvalidLocaltime)?;
        let date_time = orig_tz.from_utc_datetime(&naive_date_time);
        Ok(date_time)
    }
}

impl TryFrom<chrono::DateTime<Tz>> for Datetime {
    type Error = Errno;

    fn try_from(value: chrono::DateTime<Tz>) -> Result<Self, Self::Error> {
        let seconds = value
            .timestamp()
            .try_into()
            .map_err(|_| Errno::InvalidLocaltime)?;
        let nanoseconds = value.nanosecond();
        Ok(Datetime {
            seconds,
            nanoseconds,
        })
    }
}
