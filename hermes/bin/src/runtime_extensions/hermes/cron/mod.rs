//! Cron runtime extension implementation.
use std::{
    cmp::{max, min},
    collections::HashMap,
    fmt::{Display, Formatter},
};

use time::OffsetDateTime;

use crate::runtime_extensions::{
    bindings::{
        hermes::cron::api::{CronComponent, CronEventTag, CronSched, CronTagged, CronTime},
        wasi::clocks::monotonic_clock::Instant,
    },
    state::{Context, Stateful},
};

mod event;
mod host;

/// State
pub(crate) struct State {
    /// The crontabs hash map.
    crontabs: HashMap<CronEventTag, CronTab>,
}

impl Stateful for State {
    fn new(_ctx: &Context) -> Self {
        State {
            crontabs: HashMap::new(),
        }
    }
}

/// A crontab entry.
struct CronTab {
    /// The crontab entry.
    entry: CronTagged,
    /// When the event triggers.
    retrigger: bool,
}

/// Create a delayed crontab entry.
pub(crate) fn mkdelay_crontab(
    duration: Instant, tag: CronEventTag,
) -> wasmtime::Result<CronTagged> {
    // Add the delay to the current time as nanoseconds.
    let delayed_nanos = OffsetDateTime::now_utc().unix_timestamp_nanos() + i128::from(duration);
    let delayed = OffsetDateTime::from_unix_timestamp_nanos(delayed_nanos)?;
    let (month, day) = (delayed.month() as u8, delayed.day());
    let (hour, minute, _secs) = delayed.to_hms();
    let when = mkcron_impl(
        &vec![],
        &vec![CronComponent::At(month)],
        &vec![CronComponent::At(day)],
        &vec![CronComponent::At(hour)],
        &vec![CronComponent::At(minute)],
    );
    Ok(CronTagged { when, tag })
}

/// Convert `CronTime` arguments to a `CronSched`.
pub(crate) fn mkcron_impl(
    dow: &CronTime, month: &CronTime, day: &CronTime, hour: &CronTime, minute: &CronTime,
) -> CronSched {
    let dow_schedule: CronSched = cron_time_to_cron_sched_dow(dow);
    let month_schedule: CronSched = cron_time_to_cron_sched_month(month);
    let day_schedule: CronSched = cron_time_to_cron_sched_day(day);
    let hour_schedule: CronSched = cron_time_to_cron_sched_hour(hour);
    let minute_schedule: CronSched = cron_time_to_cron_sched_minute(minute);
    // Return the merged schedule.
    format!("{minute_schedule} {hour_schedule} {day_schedule} {month_schedule} {dow_schedule}",)
}

/// Convert a `CronTime` to a `CronSched`.
///
/// Silently clamps values within the specified `min_val..=max_val` range, removes
/// duplicates, merges overlaps, and ensures that range values are in the right order:
/// `first <= last`.
///
/// If the `CronTime` contains no components, returns `*`.
/// If the `CronTime` contains `CronComponent::All`, returns `*`.
/// If the `CronTime` contains `CronComponent::Range(first, last)`, returns `*`.
/// If the `CronTime` contains overlapping components, it merges them.
fn cron_time_to_cron_sched(cron_time: &CronTime, min_val: u8, max_val: u8) -> CronSched {
    // If vec has no components or if it includes `CronComponent::All`, skip processing and
    // return "*"
    let cron_sched: CronSched = if cron_time.is_empty() {
        format!("{}", CronComponent::All)
    } else {
        // Silently clamp values, and ensure that range values are in the right order: `first <=
        // last`. For the case of finding `CronComponent::Range((final, last))`, it is
        // replaced with `CronComponent::All`.
        let clamped: Vec<CronComponent> = clamp_cron_time_values(cron_time, min_val, max_val);
        // Merge overlapping components
        let merged: Vec<CronComponent> = merge_cron_time_overlaps(clamped);
        // Return the merged cron schedule
        merged
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<CronSched>>()
            .join(",")
    };
    cron_sched
}

/// Clamp values within the specified `min_val..=max_val` range
fn clamp_cron_time_values(
    cron_time: &[CronComponent], min_val: u8, max_val: u8,
) -> Vec<CronComponent> {
    cron_time
        .iter()
        .map(|d| d.clamp_inner(min_val, max_val))
        .collect()
}

/// Scan over the remaining components and merge them if they overlap
fn merge_cron_time_overlaps(cron_time: Vec<CronComponent>) -> Vec<CronComponent> {
    let mut cron_time = cron_time;
    // Sort the clamped components to have a consistent order
    cron_time.sort();

    let merged = cron_time
        .clone()
        .iter()
        .fold(Vec::new(), |mut out, cron_component| {
            let has_no_overlap = out
                .iter()
                .all(|&item: &CronComponent| !item.overlaps(*cron_component));
            if has_no_overlap {
                out.push(*cron_component);
            } else {
                for item in &mut out {
                    if item.overlaps(*cron_component) {
                        if let Some(merged_item) = item.merge(*cron_component) {
                            *item = merged_item;
                        }
                    }
                }
            }
            out.sort();
            out
        });
    merged
}

/// Convert a `CronTime` to a `CronSched` for the day of week.
fn cron_time_to_cron_sched_dow(cron_time: &CronTime) -> CronSched {
    cron_time_to_cron_sched(cron_time, CronComponent::MIN_DOW, CronComponent::MAX_DOW)
}

/// Convert a `CronTime` to a `CronSched` for the month.
fn cron_time_to_cron_sched_month(cron_time: &CronTime) -> CronSched {
    cron_time_to_cron_sched(
        cron_time,
        CronComponent::MIN_MONTH,
        CronComponent::MAX_MONTH,
    )
}

/// Convert a `CronTime` to a `CronSched` for the day of month.
fn cron_time_to_cron_sched_day(cron_time: &CronTime) -> CronSched {
    cron_time_to_cron_sched(cron_time, CronComponent::MIN_DAY, CronComponent::MAX_DAY)
}

/// Convert a `CronTime` to a `CronSched` for the hour of day.
fn cron_time_to_cron_sched_hour(cron_time: &CronTime) -> CronSched {
    cron_time_to_cron_sched(cron_time, CronComponent::MIN_HOUR, CronComponent::MAX_HOUR)
}

/// Convert a `CronTime` to a `CronSched` for the minute of hour.
fn cron_time_to_cron_sched_minute(cron_time: &CronTime) -> CronSched {
    cron_time_to_cron_sched(
        cron_time,
        CronComponent::MIN_MINUTE,
        CronComponent::MAX_MINUTE,
    )
}

impl CronComponent {
    /// Represents all possible values, `"*"`.
    const ALL_STR: &'static str = "*";
    /// Maximum value for `Day`.
    const MAX_DAY: u8 = 31;
    /// Maximum value for `DayOfWeek`. Sunday.
    const MAX_DOW: u8 = 7;
    /// Maximum value for `Hour`.
    const MAX_HOUR: u8 = 23;
    /// Maximum value for `Minute`.
    const MAX_MINUTE: u8 = 59;
    /// Maximum value for `Month`.
    const MAX_MONTH: u8 = 12;
    /// Minimum value for `Day`.
    const MIN_DAY: u8 = 1;
    /// Minimum value for `DayOfWeek`. Monday.
    const MIN_DOW: u8 = 0;
    /// Minimum value for `Hour`.
    const MIN_HOUR: u8 = 0;
    /// Minimum value for `Minute`.
    const MIN_MINUTE: u8 = 0;
    /// Minimum value for `Month`.
    const MIN_MONTH: u8 = 1;

    /// Clamp inner values within the given range values. Returns `CronComponent`.
    fn clamp_inner(self, first: u8, last: u8) -> Self {
        /// Implement clamping inner values within the given range values.
        fn clamp_val(val: u8, min_limit: u8, max_limit: u8) -> u8 {
            min(max(val, min_limit), max_limit)
        }

        match self {
            Self::All => self,
            Self::At(when) => Self::At(clamp_val(when, first, last)),
            Self::Range((a, b)) => {
                // Clamp values.
                let (c, d) = (clamp_val(a, first, last), clamp_val(b, first, last));
                // Ensure lowest value is first.
                let range = if c <= d { (c, d) } else { (d, c) };
                // If the range is the set as clamping limits, return `All`.
                if range == (first, last) {
                    Self::All
                } else if c == d {
                    Self::At(c)
                } else {
                    // Return the range.
                    Self::Range(range)
                }
            },
        }
    }

    /// Merge two `CronComponent` values. Returns `Option<CronComponent>`.
    ///
    /// This method makes no checks to determine if the values are within
    /// any limit.
    fn merge(self, other: CronComponent) -> Option<Self> {
        match self {
            Self::All => Some(self),
            Self::At(when) => {
                match other {
                    Self::All => Some(Self::All),
                    Self::At(w) if w == when => Some(self),
                    Self::Range((a, b)) if (a..=b).contains(&when) => Some(other),
                    _ => None,
                }
            },
            Self::Range((first, last)) => {
                match other {
                    Self::All => Some(Self::All),
                    Self::At(w) if (first..=last).contains(&w) => Some(self),
                    Self::Range((a, b)) => Some(Self::Range((min(first, a), max(last, b)))),
                    Self::At(_) => None,
                }
            },
        }
    }

    /// Determine if inner value overlaps with the argument. Returns `bool`.
    ///
    /// This method makes no checks to determine if the values are within
    /// any limit.
    fn overlaps(self, other: CronComponent) -> bool {
        match self {
            Self::All => true,
            Self::At(when) => {
                match other {
                    Self::All => true,
                    Self::At(w) => w == when,
                    Self::Range((a, b)) => (a..=b).contains(&when),
                }
            },
            Self::Range((first, last)) => {
                match other {
                    Self::All => true,
                    Self::At(w) => (first..=last).contains(&w),
                    Self::Range((a, b)) => {
                        ((first..=last).contains(&a) || (first..=last).contains(&b))
                            || ((a..=b).contains(&first) || (a..=b).contains(&last))
                    },
                }
            },
        }
    }
}

impl Display for CronComponent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => write!(f, "{}", Self::ALL_STR),
            Self::At(val) => write!(f, "{val}"),
            Self::Range((start, end)) => write!(f, "{start}-{end}"),
        }
    }
}

impl PartialEq for CronComponent {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::All => matches!(other, Self::All),
            Self::At(when) => {
                match other {
                    Self::At(w) if w == when => true,
                    Self::Range((a, b)) if (a..=b).contains(&when) => true,
                    _ => false,
                }
            },
            Self::Range((first, last)) => {
                match other {
                    Self::At(w) if first == w && last == w => true,
                    Self::Range((a, b)) if first == a && last == b => true,
                    _ => false,
                }
            },
        }
    }
}

#[allow(clippy::non_canonical_partial_ord_impl)]
/// Compare two `CronComponent`s.
///
/// `CronComponent`s are ordered in the following order, from greater to lesser:
/// - `All`
/// - `Range`
/// - `At`
impl PartialOrd for CronComponent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self {
            Self::All => {
                match other {
                    Self::All => Some(std::cmp::Ordering::Equal),
                    _ => Some(std::cmp::Ordering::Greater),
                }
            },
            Self::At(when) => {
                match other {
                    Self::At(w) => Some(when.cmp(w)),
                    _ => Some(std::cmp::Ordering::Less),
                }
            },
            Self::Range((first, last)) => {
                match other {
                    Self::All => Some(std::cmp::Ordering::Less),
                    Self::At(_) => Some(std::cmp::Ordering::Greater),
                    Self::Range((start, end)) => Some(first.cmp(start).then(last.cmp(end))),
                }
            },
        }
    }
}

impl Eq for CronComponent {}

#[allow(clippy::expect_used)]
impl Ord for CronComponent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other)
            .expect("CronComponent should always be comparable")
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use time::Duration;

    use super::*;

    // Define lower limit for the schedule component values.
    const FIRST: u8 = 1;
    // Define upper limit for the schedule component values.
    const LAST: u8 = 59;

    #[test]
    fn test_mkdelay_crontab() {
        // Get the cron schedule from the current time.
        let test_tag = "test".to_string();
        let now = OffsetDateTime::now_utc();
        let (month, day) = (now.month() as u8, now.day());
        let (hour, minute, _secs) = now.to_hms();
        let now_schedule = mkcron_impl(
            &vec![],
            &vec![CronComponent::At(month)],
            &vec![CronComponent::At(day)],
            &vec![CronComponent::At(hour)],
            &vec![CronComponent::At(minute)],
        );
        // Test the case with 0 duration
        let duration = 0u64;
        let CronTagged { when, tag } = mkdelay_crontab(duration, test_tag.clone()).unwrap();
        assert_eq!(when, now_schedule);
        assert_eq!(tag, "test");
        // Test the case with 5 minutes duration
        let minute_duration = 5u64;
        let secs_per_minute = 60u64;
        let nanos = 1_000_000_000u64;
        let duration = minute_duration * secs_per_minute * nanos;
        let then = now + Duration::minutes(minute_duration as i64);
        let (month, day) = (then.month() as u8, then.day());
        let (hour, minute, _secs) = then.to_hms();
        let then_schedule = mkcron_impl(
            &vec![],
            &vec![CronComponent::At(month)],
            &vec![CronComponent::At(day)],
            &vec![CronComponent::At(hour)],
            &vec![CronComponent::At(minute)],
        );
        let CronTagged { when, tag } = mkdelay_crontab(duration, test_tag).unwrap();
        assert_eq!(when, then_schedule);
        assert_eq!(tag, "test");
    }

    #[test]
    fn test_cron_component_overlaps() {
        assert!(CronComponent::At(1).overlaps(CronComponent::At(1)));
        assert!(!CronComponent::At(1).overlaps(CronComponent::At(2)));
        assert!(CronComponent::At(1).overlaps(CronComponent::Range((1, 2))));
        assert!(!CronComponent::At(1).overlaps(CronComponent::Range((2, 3))));
        assert!(CronComponent::At(1).overlaps(CronComponent::All));

        assert!(CronComponent::Range((1, 2)).overlaps(CronComponent::Range((1, 2))));
        assert!(CronComponent::Range((1, 2)).overlaps(CronComponent::Range((2, 3))));
        assert!(CronComponent::Range((1, 2)).overlaps(CronComponent::All));

        assert!(CronComponent::All.overlaps(CronComponent::At(1)));
        assert!(CronComponent::All.overlaps(CronComponent::Range((1, 2))));
        assert!(CronComponent::All.overlaps(CronComponent::All));
    }

    #[test]
    fn test_cron_component_merge() {
        assert_eq!(CronComponent::At(1).merge(CronComponent::At(2)), None);
        assert_eq!(
            CronComponent::At(1).merge(CronComponent::Range((1, 2))),
            Some(CronComponent::Range((1, 2)))
        );
        assert_eq!(
            CronComponent::Range((0, 1)).merge(CronComponent::Range((1, 2))),
            Some(CronComponent::Range((0, 2)))
        );
    }

    #[test]
    fn test_cron_component_order() {
        // `All` is always greater than all other `CronComponent`s.
        assert_eq!(CronComponent::All, CronComponent::All);
        assert!(CronComponent::All > CronComponent::At(0));
        assert!(CronComponent::All > CronComponent::Range((0, 0)));

        // `At(a)` is less than `All` and `Range(a, b)`
        assert!(CronComponent::At(0) < CronComponent::All);
        assert!(CronComponent::At(0) < CronComponent::Range((0, 0)));

        assert!(CronComponent::Range((0, 0)) < CronComponent::All);
        assert!(CronComponent::Range((0, 0)) > CronComponent::At(0));
        // `Range(a, b)` is equal to `Range(c, d)` if `a == c` and `b == d`.
        assert_eq!(CronComponent::Range((0, 0)), CronComponent::Range((0, 0)));
        // `Range(a, b)` is equal to `Range(c, d)` if `a == c` and `b == d`.
        assert!(CronComponent::Range((0, 0)) < CronComponent::Range((0, 1)));
    }

    #[test]
    fn test_cron_time_to_cron_sched_returns_all_if_empty() {
        let cron_schedule = cron_time_to_cron_sched(&vec![], FIRST, LAST);
        assert_eq!(cron_schedule, "*");
    }

    #[test]
    fn test_clamp_cron_time_values_within_limits() {
        // Components with values outside the clamping limits
        let cron_schedule = clamp_cron_time_values(&[CronComponent::At(0)], FIRST, LAST);
        assert_eq!(cron_schedule, vec![CronComponent::At(FIRST)]);

        let cron_schedule = clamp_cron_time_values(&[CronComponent::At(100)], FIRST, LAST);
        assert_eq!(cron_schedule, vec![CronComponent::At(LAST)]);

        let cron_schedule = clamp_cron_time_values(&[CronComponent::Range((62, 64))], FIRST, LAST);
        assert_eq!(cron_schedule, vec![CronComponent::At(LAST)]);

        let cron_schedule = clamp_cron_time_values(&[CronComponent::Range((0, 20))], FIRST, LAST);
        assert_eq!(cron_schedule, vec![CronComponent::Range((FIRST, 20))]);

        let cron_schedule = clamp_cron_time_values(&[CronComponent::Range((0, 200))], FIRST, LAST);
        assert_eq!(cron_schedule, vec![CronComponent::All]);

        let cron_schedule =
            clamp_cron_time_values(&[CronComponent::Range((FIRST, LAST))], FIRST, LAST);
        assert_eq!(cron_schedule, vec![CronComponent::All]);
    }

    #[test]
    fn test_merge_cron_time_overlaps() {
        // `CronTime`s that contain `All` removes everything else.
        let cron_schedule =
            merge_cron_time_overlaps(vec![CronComponent::At(3), CronComponent::All]);
        assert_eq!(cron_schedule, vec![CronComponent::All]);

        let cron_schedule = merge_cron_time_overlaps(vec![CronComponent::All, CronComponent::All]);
        assert_eq!(cron_schedule, vec![CronComponent::All]);

        let cron_schedule = merge_cron_time_overlaps(vec![
            CronComponent::At(5),
            CronComponent::At(5),
            CronComponent::Range((5, 5)),
            CronComponent::Range((5, 5)),
        ]);
        assert_eq!(cron_schedule, vec![CronComponent::At(5)]);

        let cron_schedule = merge_cron_time_overlaps(vec![
            CronComponent::At(7),
            CronComponent::Range((5, 30)),
            CronComponent::Range((5, 55)),
        ]);
        assert_eq!(cron_schedule, vec![CronComponent::Range((5, 55))]);

        let cron_schedule = merge_cron_time_overlaps(vec![
            CronComponent::Range((10, 15)),
            CronComponent::Range((14, 25)),
            CronComponent::Range((5, 15)),
        ]);
        assert_eq!(cron_schedule, vec![CronComponent::Range((5, 25))]);
    }

    #[test]
    fn test_cron_time_to_cron_sched_orders_components() {
        let cron_schedule = cron_time_to_cron_sched(
            &vec![
                CronComponent::Range((2, 4)),
                CronComponent::At(1),
                CronComponent::Range((6, 7)),
                CronComponent::At(8),
                CronComponent::Range((9, 10)),
                CronComponent::At(11),
            ],
            FIRST,
            LAST,
        );
        assert_eq!(cron_schedule, "1,8,11,2-4,6-7,9-10");
    }

    #[test]
    fn test_mkcron_impl() {
        // Test empty `CronTime`s
        assert_eq!(
            mkcron_impl(&vec![], &vec![], &vec![], &vec![], &vec![]),
            "* * * * *"
        );
        // Test clamp values use `CronComponent` constants
        assert_eq!(
            mkcron_impl(
                &vec![CronComponent::At(100)],
                &vec![CronComponent::At(100)],
                &vec![CronComponent::At(100)],
                &vec![CronComponent::At(100)],
                &vec![CronComponent::At(100)]
            ),
            format!(
                "{} {} {} {} {}",
                CronComponent::MAX_MINUTE,
                CronComponent::MAX_HOUR,
                CronComponent::MAX_DAY,
                CronComponent::MAX_MONTH,
                CronComponent::MAX_DOW
            )
        );
        assert_eq!(
            mkcron_impl(
                &vec![CronComponent::At(0)],
                &vec![CronComponent::At(0)],
                &vec![CronComponent::At(0)],
                &vec![CronComponent::At(0)],
                &vec![CronComponent::At(0)]
            ),
            format!(
                "{} {} {} {} {}",
                CronComponent::MIN_MINUTE,
                CronComponent::MIN_HOUR,
                CronComponent::MIN_DAY,
                CronComponent::MIN_MONTH,
                CronComponent::MIN_DOW
            )
        );
    }
}
