//! Cron host implementation for WASM runtime.
use std::{
    cmp::{max, min},
    fmt::{Display, Formatter},
};

use crate::{
    runtime_extensions::{
        bindings::{
            hermes::cron::api::{
                CronComponent, CronEventTag, CronSched, CronTagged, CronTime, Host,
            },
            wasi::clocks::monotonic_clock::Instant,
        },
        hermes::cron::CronTab,
    },
    state::HermesState,
};

impl Host for HermesState {
    /// # Schedule Recurrent CRON event
    ///
    /// Cron events will be delivered to the `on-cron` event handler.
    ///
    /// ## Parameters
    ///
    /// - `entry`: The crontab entry to add.
    /// - `when`: When the event triggers.  Standard crontab format.
    /// - `tag`: A tag which will accompany the triggered event.
    /// - `retrigger`:
    /// - `true`: The event will re-trigger every time the crontab entry matches until
    ///   cancelled.
    /// - `false`: The event will automatically cancel after it is generated once.
    ///
    /// ## Returns
    ///
    /// - `true`: Crontab added successfully.  (Or the crontab event already exists)
    /// - `false`: Crontab failed to be added.
    ///
    /// ## Note:
    ///
    /// If the crontab entry already exists, the retrigger flag can be changed by calling
    /// this function.  This could be useful where a retriggering crontab event is desired
    /// to be stopped, but ONLY after it has triggered once more.
    fn add(&mut self, entry: CronTagged, retrigger: bool) -> wasmtime::Result<bool> {
        self.hermes
            ._cron
            .crontabs
            .insert(entry.tag.clone(), CronTab { entry, retrigger });
        Ok(true)
    }

    /// # Schedule A Single cron event after a fixed delay.
    ///
    /// Allows for easy timed wait events to be delivered without
    /// requiring datetime calculations or formatting cron entries.
    ///
    /// ## Parameters
    ///
    /// - `duration`: How many nanoseconds to delay.  The delay will be AT LEAST this
    ///   long.
    /// - `tag`: A tag which will accompany the triggered event.
    ///
    /// ## Returns
    ///
    /// - `true`: Crontab added successfully.
    /// - `false`: Crontab failed to be added.
    ///
    /// ## Note:
    ///
    /// This is a convenience function which will automatically calculate the crontab
    /// entry needed to trigger the event after the requested `duration`.
    /// It is added as a non-retriggering event.
    /// Listing the crontabs after this call will list the delay in addition to all other
    /// crontab entries.
    fn delay(&mut self, duration: Instant, tag: CronEventTag) -> wasmtime::Result<bool> {
        todo!()
    }

    /// # List currently active cron schedule.
    ///
    /// Allows for management of scheduled cron events.
    ///
    /// ## Parameters
    ///
    /// - `tag`: Optional, the tag to limit the list to.  If `none` then all crons listed.
    ///
    /// ## Returns
    ///
    /// - A list of tuples containing the scheduled crontabs and their tags, along with
    ///   the current retrigger flag.
    /// The list is sorted from most crontab that will trigger soonest to latest.
    /// Crontabs are only listed once, in the case where a crontab may be scheduled
    /// may times before a later one.
    /// - `0` - `cron-tagged` - The Tagged crontab event.
    /// - `1` - `bool` - The state of the retrigger flag.
    fn ls(&mut self, tag: Option<CronEventTag>) -> wasmtime::Result<Vec<(CronTagged, bool)>> {
        if let Some(tag) = tag {
            match self.hermes._cron.crontabs.get(&tag) {
                Some(cron) => Ok(vec![(cron.entry.clone(), cron.retrigger)]),
                None => Ok(vec![]),
            }
        } else {
            Ok(self
                .hermes
                ._cron
                .crontabs
                .values()
                .map(|cron| (cron.entry.clone(), cron.retrigger))
                .collect())
        }
    }

    /// # Remove the requested crontab.
    ///
    /// Allows for management of scheduled cron events.
    ///
    /// ## Parameters
    ///
    /// - `when`: The crontab entry to add.  Standard crontab format.
    /// - `tag`: A tag which will accompany the triggered event.
    ///
    /// ## Returns
    ///
    /// - `true`: The requested crontab was deleted and will not trigger.
    /// - `false`: The requested crontab does not exist.
    fn rm(&mut self, entry: CronTagged) -> wasmtime::Result<bool> {
        match self.hermes._cron.crontabs.remove(&entry.tag) {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    /// # Make a crontab entry from individual time values.
    ///
    /// Creates the properly formatted cron entry
    /// from numeric cron time components.
    /// Convenience function to make building cron strings simpler when they are
    /// calculated from data.
    ///
    /// ## Parameters
    ///
    /// - `dow` - `DayOfWeek` (0-7, 0 or 7 = Sunday)
    /// - `month` - Month of the year (1-12, 1 = January)
    /// - `day` - Day in the month (1-31)
    /// - `hour` - Hour in the day (0-23)
    /// - `minute` - Minute in the hour (0-59)
    ///
    /// ## Returns
    ///
    /// - A matching `cron-sched` ready for use in the cron functions above.
    ///
    /// ## Note:
    /// No checking is done to determine if the requested date is valid.
    /// If a particular component is out of its allowable range it will be silently
    /// clamped within the allowable range of each parameter.
    /// Redundant entries will be removed.
    /// - For example specifying a `month` as `3` and `2-4` will
    /// remove the individual month and only produce the range.
    fn mkcron(
        &mut self, dow: CronTime, month: CronTime, day: CronTime, hour: CronTime, minute: CronTime,
    ) -> wasmtime::Result<CronSched> {
        let dow_schedule: CronSched =
            cron_time_to_cron_sched(&dow, CronComponent::MIN_DOW, CronComponent::MAX_DOW);
        let month_schedule: CronSched =
            cron_time_to_cron_sched(&month, CronComponent::MIN_MONTH, CronComponent::MAX_MONTH);
        let day_schedule: CronSched =
            cron_time_to_cron_sched(&day, CronComponent::MIN_DAY, CronComponent::MAX_DAY);
        let hour_schedule: CronSched =
            cron_time_to_cron_sched(&hour, CronComponent::MIN_HOUR, CronComponent::MAX_HOUR);
        let minute_schedule: CronSched = cron_time_to_cron_sched(
            &minute,
            CronComponent::MIN_MINUTE,
            CronComponent::MAX_MINUTE,
        );
        let cron_sched = format!(
            "{minute_schedule} {hour_schedule} {day_schedule} {month_schedule} {dow_schedule}",
        );
        Ok(cron_sched)
    }
}

/// Convert a `CronTime` to a `CronSched`.
///
/// Silently clamps values, removes duplicates, and ensures that range values are
/// in the right order: `first <= last`.
/// If the `CronTime` contains no components, returns `*`.
/// If the `CronTime` contains `CronComponent::All`, returns `*`.
/// If the `CronTime` contains `CronComponent::Range(first, last)`, returns `*`.
/// If the `CronTime` contains overlapping components, it merges them.
///
/// Example:
///
/// ```
/// use hermes::runtime::host::hermes::cron::{cron_time_to_cron_sched, CronComponent};
///
/// let cron_time = vec![
///     CronComponent::All,
///     CronComponent::Range((2, 4)),
///     CronComponent::At(5),
/// ];
///
/// let dow_schedule =
///     cron_time_to_cron_sched(&cron_time, CronComponent::MIN_DOW, CronComponent::MAX_DOW);
/// assert_eq!(dow_schedule, "*");
///
/// let cron_time = vec![
///     CronComponent::Range((2, 4)),
///     CronComponent::At(5),
///     CronComponent::Range((6, 7)),
///     CronComponent::At(8),
///     CronComponent::Range((9, 10)),
///     CronComponent::At(11),
/// ];
///
/// let dow_schedule =
///     cron_time_to_cron_sched(&cron_time, CronComponent::MIN_DOW, CronComponent::MAX_DOW);
/// assert_eq!(dow_schedule, "5,8,11,2-4,6-7,9-10");
/// ```
fn cron_time_to_cron_sched(cron_time: &CronTime, min_val: u8, max_val: u8) -> CronSched {
    // If vec has no components or if it includes `CronComponent::All`, skip processing and
    // return "*"
    let cron_sched: CronSched = if cron_time.is_empty() {
        format!("{}", CronComponent::All)
    } else {
        // Silently clamp values, and ensure that range values are in the right order: `first <=
        // last`. For the case of finding `CronComponent::Range((final, last))`, it is
        // replaced with `CronComponent::All`.
        let mut clamped: Vec<CronComponent> = cron_time
            .iter()
            .map(|d| d.clamp_inner(min_val, max_val))
            .collect();

        clamped.sort();

        let clamped_len = clamped.len();
        let mut deduped: CronTime = clamped.clone().iter_mut().enumerate().fold(
            Vec::new(),
            |mut out, (i, cron_component)| {
                let idx = i + 1;
                if let Some(remaining) = clamped.get_mut(idx..clamped_len) {
                    let not_downstream = remaining
                        .iter()
                        .all(|other| !other.contains(*cron_component));
                    if not_downstream {
                        // Push the current cron component
                        out.push(*cron_component);
                    }
                }
                out
            },
        );

        let deduped_len = deduped.len();
        let merged = deduped.clone().iter_mut().enumerate().fold(
            Vec::new(),
            |mut out, (i, cron_component)| {
                let idx = i + 1;
                if let Some(remaining) = deduped.get_mut(idx..deduped_len) {
                    let no_overlap = remaining
                        .iter()
                        .all(|other| !other.overlaps(*cron_component));
                    if no_overlap {
                        // Push the current cron component
                        out.push(*cron_component);
                    } else {
                        for other in remaining.iter_mut() {
                            // Check if the cron components overlap
                            if cron_component.overlaps(*other) {
                                // Merge the two cron components
                                if let Some(merged) = cron_component.merge(*other) {
                                    *other = merged;
                                    // Once merged, stop iterating over the remaining cron
                                    // components
                                    break;
                                }
                            }
                        }
                    }
                }
                out
            },
        );
        merged
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<CronSched>>()
            .join(",")
    };
    cron_sched
}

impl CronComponent {
    /// Represents all possible values, `"*"`.
    const ALL_STR: &'static str = "*";
    /// Maximum value for `Day`.
    const MAX_DAY: u8 = 31;
    /// Maximum value for `DayOfWeek`.
    const MAX_DOW: u8 = 7;
    /// Maximum value for `Hour`.
    const MAX_HOUR: u8 = 23;
    /// Maximum value for `Minute`.
    const MAX_MINUTE: u8 = 59;
    /// Maximum value for `Month`.
    const MAX_MONTH: u8 = 12;
    /// Minimum value for `Day`.
    const MIN_DAY: u8 = 1;
    /// Minimum value for `DayOfWeek`.
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

    /// Determine if inner value includes the argument. Returns `bool`.
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
                    Self::Range((a, b))
                        if (first..=last).contains(&a) || (first..=last).contains(&b) =>
                    {
                        Some(Self::Range((min(first, a), max(last, b))))
                    },
                    _ => None,
                }
            },
        }
    }

    /// Determine if inner value includes the argument. Returns `bool`.
    fn contains(self, other: CronComponent) -> bool {
        match self {
            Self::All => true,
            Self::At(when) => matches!(other, Self::At(w) if w == when),
            Self::Range((first, last)) => {
                match other {
                    Self::All => true,
                    Self::At(w) => (first..=last).contains(&w),
                    Self::Range((a, b)) => {
                        (first..=last).contains(&a) && (first..=last).contains(&b)
                    },
                }
            },
        }
    }

    /// Determine if inner value overlaps with the argument. Returns `bool`.
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
                        (first..=last).contains(&a) || (first..=last).contains(&b)
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
/// Example:
///
/// ```rust
/// # use hermes::runtime::host::hermes::cron::*;
/// assert!(CronComponent::At(1) < CronComponent::At(2));
/// assert!(CronComponent::At(1) < CronComponent::Range((2, 3)));
/// assert!(CronComponent::At(1) < CronComponent::All);
/// assert!(CronComponent::Range((1, 2)) < CronComponent::Range((3, 4)));
/// assert!(CronComponent::Range((1, 2)) < CronComponent::All);
/// assert!(CronComponent::All == CronComponent::All);
/// assert!(CronComponent::All > CronComponent::Range((1, 2)));
/// assert!(CronComponent::All > CronComponent::At(1));
/// ```
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
