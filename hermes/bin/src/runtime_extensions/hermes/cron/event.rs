//! Cron runtime extension event handler implementation.

use anyhow::Error;
use chrono::Utc;
use saffron::Cron;

use super::state::cron_queue_rm;
use crate::{
    event::HermesEventPayload,
    runtime_extensions::bindings::{hermes::cron::api::CronTagged, unchecked_exports},
};

unchecked_exports::define! {
    /// Extends [`wasmtime::component::Instance`] with guest functions for cron.
    trait ComponentInstanceExt {
        #[wit("hermes:cron/event", "on-cron")]
        fn hermes_cron_event_on_cron(event: &CronTagged, last: bool) -> bool;
    }
}

/// Duration in nanoseconds used for the Cron Service.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub(crate) struct CronDuration(u64);

impl CronDuration {
    /// Saturating integer subtraction. Computes `self - rhs`, saturating
    /// at the numeric bounds instead of overflowing.
    #[must_use = "this returns the result of the operation, \
                    without modifying the original"]
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub const fn saturating_sub(
        self,
        rhs: Self,
    ) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl TryFrom<i64> for CronDuration {
    type Error = Error;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        let duration: u64 = value
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid Timestamp"))?;
        Ok(CronDuration(duration))
    }
}

impl From<CronDuration> for u64 {
    fn from(value: CronDuration) -> Self {
        value.0
    }
}

impl From<u64> for CronDuration {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

/// On cron event
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub(crate) struct OnCronEvent {
    /// The tagged cron event that was triggered.
    pub(crate) tag: CronTagged,
    /// This cron event will not retrigger.
    pub(crate) last: bool,
}

impl HermesEventPayload for OnCronEvent {
    fn event_name(&self) -> &'static str {
        "on-cron"
    }

    fn execute(
        &self,
        module: &mut crate::wasm::module::ModuleInstance,
    ) -> anyhow::Result<()> {
        let res =
            module
                .instance
                .hermes_cron_event_on_cron(&mut module.store, &self.tag, self.last)?;
        // if the response is `false`, check if the event would
        // re-trigger, if so, remove it.
        if !res && !self.last {
            let app_name = module.store.data().app_name();
            cron_queue_rm(app_name, self.tag.clone());
        }
        Ok(())
    }
}

impl OnCronEvent {
    /// Get the next scheduled cron event after the optional start timestamp, or after the
    /// current timestamp.
    ///
    /// # Parameters
    ///
    /// * `start: Option<CronDuration>` - The optional start timestamp. If `None`, the
    ///   current time is used.
    ///
    /// # Returns
    ///
    /// * `Some(CronDuration)` - The next timestamp for the `OnCronEvent`.
    /// * `None` if the timestamp could not be calculated.
    pub(crate) fn tick_after(
        &self,
        start: Option<CronDuration>,
    ) -> Option<CronDuration> {
        let cron = self.cron()?;
        if cron.any() {
            let datetime = Self::start_datetime(start)?;
            let cdt = cron.iter_after(datetime).next()?;
            let timestamp = cdt.timestamp_nanos_opt()?;
            timestamp.try_into().ok()
        } else {
            None
        }
    }

    /// Get the next scheduled cron event from the optional start timestamp, or from the
    /// current timestamp.
    ///
    /// # Parameters
    ///
    /// * `start: Option<CronDuration>` - The optional start timestamp. If `None`, the
    ///   current time is used.
    ///
    /// # Returns
    ///
    /// * `Some(CronDuration)` - The next timestamp for the `OnCronEvent`.
    /// * `None` if the timestamp could not be calculated.
    pub(crate) fn tick_from(
        &self,
        start: Option<CronDuration>,
    ) -> Option<CronDuration> {
        let cron = self.cron()?;
        if cron.any() {
            let datetime = Self::start_datetime(start)?;
            let cdt = cron.iter_from(datetime).next()?;
            let timestamp = cdt.timestamp_nanos_opt()?;
            timestamp.try_into().ok()
        } else {
            None
        }
    }

    /// Get the `Cron` from the inner `CronSchedule`.
    fn cron(&self) -> Option<Cron> {
        let when = &self.tag.when;
        when.parse::<Cron>().ok()
    }

    /// Get the UTC datetime from an optional start timestamp.
    ///
    /// Use the `start` timestamp if provided, otherwise use the current time.
    ///
    /// Returns `None` if the datetime could not be calculated.
    fn start_datetime(start: Option<CronDuration>) -> Option<chrono::DateTime<Utc>> {
        let datetime = match start {
            None => Utc::now(),
            Some(ts) => chrono::DateTime::from_timestamp_nanos(u64::from(ts).try_into().ok()?),
        };
        Some(datetime)
    }
}

impl PartialEq for CronTagged {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.tag == other.tag && self.when == other.when
    }
}

impl Eq for CronTagged {}

impl PartialOrd for CronTagged {
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CronTagged {
    fn cmp(
        &self,
        other: &Self,
    ) -> std::cmp::Ordering {
        self.when.cmp(&other.when).then(self.tag.cmp(&other.tag))
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use chrono::prelude::*;

    use super::*;

    #[test]
    fn test_cron_queue() {
        let start = NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_nano_opt(0, 0, 0, 0)
            .unwrap();
        let datetime = DateTime::from_naive_utc_and_offset(start, Utc);

        let cron: Cron = "* * * * *".parse().unwrap();
        for time in cron.clone().iter_from(datetime).enumerate().take(5) {
            // generates
            // 1970-01-01 00:00:00 UTC
            // 1970-01-01 00:00:01 UTC
            // 1970-01-01 00:00:02 UTC
            // 1970-01-01 00:00:03 UTC
            // 1970-01-01 00:00:04 UTC
            assert_eq!(
                time.1,
                Utc.with_ymd_and_hms(1970, 1, 1, 0, (time.0).try_into().unwrap(), 0)
                    .unwrap()
            );
        }

        let cron: Cron = "0 0 * * *".parse().unwrap();
        for time in cron.clone().iter_from(datetime).enumerate().take(5) {
            // generates
            // 1970-01-01 00:00:00 UTC
            // 1970-01-02 00:00:00 UTC
            // 1970-01-03 00:00:00 UTC
            // 1970-01-04 00:00:00 UTC
            // 1970-01-05 00:00:00 UTC
            assert_eq!(
                time.1,
                Utc.with_ymd_and_hms(1970, 1, 1 + u32::try_from(time.0).unwrap(), 0, 0, 0)
                    .unwrap()
            );
        }

        // Every first day of the month
        let cron: Cron = "0 0 1 * *".parse().unwrap();
        for time in cron.clone().iter_from(datetime).enumerate().take(5) {
            // generates
            // 1970-01-01 00:00:00 UTC
            // 1970-02-01 00:00:00 UTC
            // 1970-03-01 00:00:00 UTC
            // 1970-04-01 00:00:00 UTC
            // 1970-05-01 00:00:00 UTC
            assert_eq!(
                time.1,
                Utc.with_ymd_and_hms(1970, 1 + u32::try_from(time.0).unwrap(), 1, 0, 0, 0)
                    .unwrap()
            );
        }

        // Every first day of January
        let cron: Cron = "0 0 1 1 *".parse().unwrap();
        for time in cron.clone().iter_from(datetime).enumerate().take(5) {
            // generates
            // 1970-01-01 00:00:00 UTC
            // 1971-01-01 00:00:00 UTC
            // 1972-01-01 00:00:00 UTC
            // 1973-01-01 00:00:00 UTC
            // 1974-01-01 00:00:00 UTC
            assert_eq!(
                time.1,
                Utc.with_ymd_and_hms(1970 + i32::try_from(time.0).unwrap(), 1, 1, 0, 0, 0)
                    .unwrap()
            );
        }

        // every monday and the first day of January
        let cron: Cron = "0 0 1 1 7".parse().unwrap();
        let times: Vec<DateTime<Utc>> = cron.clone().iter_from(datetime).take(5).collect();
        assert_eq!(
            times,
            vec![
                Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(1970, 1, 3, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(1970, 1, 10, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(1970, 1, 17, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(1970, 1, 24, 0, 0, 0).unwrap(),
            ]
        );

        let cron: Cron = "0 0 1 1,3,5,7,9,11 *".parse().unwrap();
        let times: Vec<DateTime<Utc>> = cron.clone().iter_from(datetime).take(5).collect();
        assert_eq!(
            times,
            vec![
                Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(1970, 3, 1, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(1970, 5, 1, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(1970, 7, 1, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(1970, 9, 1, 0, 0, 0).unwrap(),
            ]
        );
    }
}
