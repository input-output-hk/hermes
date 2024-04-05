//! Localtime runtime extension implementation.

use chrono::{Datelike, NaiveDate, TimeZone, Timelike};
use chrono_tz::{OffsetName, Tz};

use crate::runtime_extensions::bindings::{
    hermes::localtime::api::{Errno, Localtime, Timezone},
    wasi::clocks::wall_clock::Datetime,
};

mod host;
mod localtime;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}

/// Get `Tz` from an optional `Timezone`.
///
/// If present, `tz` is parsed into `Tz`, otherwise the system timezone is
/// returned.
///
/// This function returns `Errno::UnknownTimezone` if `tz` cannot be parsed.
///
/// **Parameters**
/// * `tz`: `Option<Timezone>`.
///
/// **Returns**
/// `wasmtime::Result<Result<Tz, Errno>>`.
pub(crate) fn get_tz(tz: Option<Timezone>) -> Result<Tz, Errno> {
    let timezone_str = match tz {
        Some(tz_str) => tz_str,
        None => iana_time_zone::get_timezone().map_err(|_| Errno::UnknownTimezone)?,
    };
    timezone_str.parse().map_err(|_| Errno::UnknownTimezone)
}

impl TryFrom<chrono::DateTime<Tz>> for Localtime {
    type Error = Errno;

    fn try_from(dt: chrono::DateTime<Tz>) -> Result<Self, Self::Error> {
        let localtime = Localtime {
            year: dt.year().try_into().map_err(|_| Errno::YearOutOfRange)?,
            month: dt.month().try_into().map_err(|_| Errno::InvalidLocaltime)?,
            dow: dt
                .weekday()
                .number_from_monday()
                .try_into()
                .map_err(|_| Errno::InvalidLocaltime)?,
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
            tz: dt.offset().tz_id().to_string(),
        };
        Ok(localtime)
    }
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

impl TryFrom<Localtime> for Datetime {
    type Error = Errno;

    fn try_from(value: Localtime) -> Result<Self, Self::Error> {
        let dt: chrono::DateTime<Tz> = value.try_into()?;
        dt.try_into()
    }
}
