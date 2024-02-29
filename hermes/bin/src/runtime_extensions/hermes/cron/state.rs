//! Cron State.
/// The crontabs hash map.
use std::{
    collections::BTreeSet,
    hash::{Hash, Hasher},
    process::exit,
    sync::{mpsc, Arc, Mutex},
    thread::JoinHandle,
};

use chrono::Utc;
use dashmap::DashMap;
//  std::sync::LazyLock is still unstable
use once_cell::sync::Lazy;
use saffron::Cron;

use super::event::OnCronEvent;
use crate::runtime_extensions::{
    bindings::hermes::cron::api::{CronEventTag, CronTagged, Instant},
    hermes::cron::mkdelay_crontab,
};

/// Cron Internal State
pub(crate) static CRON_INTERNAL_STATE: Lazy<Mutex<InternalState>> =
    Lazy::new(|| Mutex::new(InternalState::new()));

/// Name of the Application that owns the `OnCronEvent`s.
type AppName = String;

/// Storage for application-specific crontabs.
type AppCronState = DashMap<CronTagged, OnCronEvent>;

/// Storage for the crontabs.
type CronTabStorage = DashMap<AppName, AppCronState>;

/// Timestamp for when to run the cron.
type CronTimestamp = u64;

/// Scheduled Date and Time for sending a cron event.
struct ScheduledCron {
    /// Scheduled time for running the event.
    _timestamp: CronTimestamp,
    /// The crontab event.
    _event: OnCronEvent,
}

impl OnCronEvent {
    /// Get the next scheduled cron event after (excluding) an optional start timestamp.
    ///
    /// # Parameters
    ///
    /// * `start: Option<CronTimestamp>` - The optional start timestamp. If `None`, the
    ///   current time is used.
    ///
    /// # Returns
    ///
    /// * `Some(CronTimestamp)` - The next timestamp for the `OnCronEvent`.
    /// * `None` if the timestamp could not be calculated.
    fn _after(&self, start: Option<CronTimestamp>) -> Option<CronTimestamp> {
        let cron = self._cron()?;
        if cron.any() {
            let datetime = Self::_start_datetime(start)?;
            let cdt = cron.iter_after(datetime).next()?;
            let timestamp = cdt.timestamp_nanos_opt()?;
            timestamp.try_into().ok()
        } else {
            None
        }
    }

    /// Get the next scheduled cron event from (including) an optional start timestamp.
    ///
    /// # Parameters
    ///
    /// * `start: Option<CronTimestamp>` - The optional start timestamp. If `None`, the
    ///   current time is used.
    ///
    /// # Returns
    ///
    /// * `Some(CronTimestamp)` - The next timestamp for the `OnCronEvent`.
    /// * `None` if the timestamp could not be calculated.
    fn _from(&self, start: Option<CronTimestamp>) -> Option<CronTimestamp> {
        let cron = self._cron()?;
        if cron.any() {
            let datetime = Self::_start_datetime(start)?;
            let cdt = cron.iter_from(datetime).next()?;
            let timestamp = cdt.timestamp_nanos_opt()?;
            timestamp.try_into().ok()
        } else {
            None
        }
    }

    /// Get the cron.
    fn _cron(&self) -> Option<Cron> {
        let when = &self.tag.when;
        when.parse::<Cron>().ok()
    }

    /// Get the UTC datetime from an optional start timestamp.
    ///
    /// Use the `start` timestamp if provided, otherwise use the current time.
    ///
    /// Returns `None` if the datetime could not be calculated.:w
    fn _start_datetime(start: Option<CronTimestamp>) -> Option<chrono::DateTime<Utc>> {
        let datetime = match start {
            None => Utc::now(),
            Some(dt) => {
                let dt = chrono::NaiveDateTime::from_timestamp_nanos(dt.try_into().ok()?)?;
                chrono::DateTime::from_naive_utc_and_offset(dt, Utc)
            },
        };
        Some(datetime)
    }
}

/// Internal State.
pub(crate) struct InternalState {
    /// The storage for crontabs.
    storage: CronTabStorage,
    /// The send events to the crontab queue.
    _cron_queue: mpsc::Sender<ScheduledCron>,
    /// The crontab queue task runs in the background.
    _task: CronQueueTask,
}

/// The crontab queue task runs in the background.
struct CronQueueTask {
    /// Handle to the crontab queue task.
    _task: JoinHandle<()>,
}

impl CronQueueTask {
    /// Create a new `CronQueueTask`.
    fn new(_receiver: &Arc<Mutex<mpsc::Receiver<ScheduledCron>>>) -> Self {
        let handle = std::thread::spawn(|| {
            println!("wip: cron queue task goes here");
            exit(0);
        });
        Self { _task: handle }
    }
}

impl InternalState {
    /// Create a new `InternalState`.
    pub(crate) fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            storage: CronTabStorage::default(),
            _cron_queue: sender,
            _task: CronQueueTask::new(&Arc::new(Mutex::new(receiver))),
        }
    }

    /// Add a new crontab entry.
    ///
    /// Allows for management of scheduled cron events queue.
    ///
    /// Cron events will be delivered to the `on-cron` event handler.
    ///
    /// ## Parameters
    ///
    /// - `app_name`:  `AppName`. The name of the application that owns the crontab.
    /// - `entry`:  `CronTagged`. The crontab entry to add.
    /// - `retrigger`:  `bool`. If `true`, the event will re-trigger every time the
    ///   crontab entry matches until cancelled.
    ///
    /// ## Returns
    ///
    /// - `true`: Crontab added successfully.
    /// - `false`: Crontab failed to be added.
    pub(crate) fn add_crontab(&self, app_name: &str, entry: CronTagged, retrigger: bool) -> bool {
        let tagged = entry.clone();
        let crontab = OnCronEvent {
            tag: entry,
            last: retrigger,
        };
        if let Some(app_cron) = self.storage.get_mut(app_name) {
            app_cron.insert(tagged, crontab);
        } else {
            let app_cron: DashMap<CronTagged, OnCronEvent> = DashMap::new();
            app_cron.insert(tagged, crontab);
            self.storage.insert(AppName::from(app_name), app_cron);
        }
        // todo!("implement cron event queue")
        true
    }

    /// Schedule a single cron event after a fixed delay.
    ///
    /// Allows for easy timed wait events to be delivered without
    /// requiring datetime calculations or formatting cron entries.
    ///
    /// ## Parameters
    ///
    /// - `app_name`:  `AppName`. The name of the application that owns the crontab.
    /// - `duration`: `Instant`. How many nanoseconds to delay.  The delay will be AT
    ///   LEAST this long.
    /// - `tag`:  `CronEventTag`. A tag which will accompany the triggered event.
    ///
    /// ## Returns
    ///
    /// - `Ok(true)`: Crontab added successfully.
    /// - `Ok(false)`: Crontab failed to be added.
    /// - `Err`: Returns error if the duration is invalid for generating a crontab entry.
    pub(crate) fn delay_crontab(
        &self, app_name: &str, duration: Instant, tag: CronEventTag,
    ) -> wasmtime::Result<bool> {
        let crontagged = mkdelay_crontab(duration, tag)?;
        self.add_crontab(app_name, crontagged, false);
        // todo!("implement cron event queue")
        Ok(true)
    }

    /// List crontabs for an application.
    ///
    /// Allows for management of scheduled cron events queue.
    /// If `tag` is `none` then all crontabs are listed.
    /// Otherwise, only the crontabs with the specified tag are listed.
    ///
    /// ## Parameters
    ///
    /// - `tag`: Optional, the tag to limit the list to.  If `none` then all crons listed.
    ///
    /// ## Returns
    ///
    /// - A list of tuples containing the scheduled crontabs and their tags, along with
    ///   the current retrigger flag.  `Vec<(CronEventTag, bool)>`
    /// The list is sorted from most crontab that will trigger soonest to latest.
    /// Crontabs are only listed once, in the case where a crontab may be scheduled
    /// may times before a later one.
    pub(crate) fn ls_crontabs(
        &self, app_name: &str, tag: Option<CronEventTag>,
    ) -> Vec<(CronTagged, bool)> {
        if let Some(app_cron) = self.storage.get_mut(app_name) {
            if let Some(tag) = tag {
                app_cron
                    .iter()
                    .filter(|cron| cron.tag.tag == tag)
                    .map(|cron| (cron.tag.clone(), cron.last))
                    .collect::<BTreeSet<(CronTagged, bool)>>()
                    .into_iter()
                    .collect()
            } else {
                app_cron
                    .iter()
                    .map(|cron| (cron.tag.clone(), cron.last))
                    .collect::<BTreeSet<(CronTagged, bool)>>()
                    .into_iter()
                    .collect()
            }
        } else {
            vec![]
        }
    }

    /// Remove the requested crontab.
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
    pub(crate) fn rm_crontab(&self, app_name: &str, entry: &CronTagged) -> bool {
        if let Some(app_cron) = self.storage.get_mut(app_name) {
            app_cron.remove(entry).is_some()
        } else {
            false
        }
    }
}

impl Hash for CronTagged {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tag.hash(state);
        self.when.hash(state);
    }
}

impl PartialEq for CronTagged {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag && self.when == other.when
    }
}

impl Eq for CronTagged {}

impl PartialOrd for CronTagged {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(std::cmp::Ord::cmp(self, other))
    }
}

#[allow(clippy::expect_used)]
impl Ord for CronTagged {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other)
            .expect("CronTagged should always be comparable")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_cron_state() {
        let state = CRON_INTERNAL_STATE.lock().unwrap();
        let app_name = "test";
        // Initial state for any AppName is always empty
        assert!(state.ls_crontabs(app_name, None).is_empty());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_cron_state_add_crontab() {
        let state = CRON_INTERNAL_STATE.lock().unwrap();
        let app_name = "test";
        let every_minute_when = "* * * * *";
        let every_day_when = "0 0 * * *";
        let every_month_when = "0 0 1 * *";
        let example_tag = "ExampleTag";
        let other_tag = "OtherTag";
        let crontab_example_1 = CronTagged {
            // triggers every minute
            when: every_minute_when.into(),
            tag: example_tag.into(),
        };
        let crontab_example_2 = CronTagged {
            // triggers every minute
            when: every_month_when.into(),
            tag: example_tag.into(),
        };
        let _crontab_example_3 = CronTagged {
            // triggers every minute
            when: every_day_when.into(),
            tag: example_tag.into(),
        };
        let _crontab_other_1 = CronTagged {
            // triggers every minute
            when: every_minute_when.into(),
            tag: other_tag.into(),
        };
        let retrigger_yes = true;
        let retrigger_no = false;
        // Initial state for any AppName is always empty
        assert!(state.ls_crontabs(app_name, None).is_empty());
        assert!(state.add_crontab(app_name, crontab_example_1.clone(), retrigger_yes));
        // re-inserting returns true
        assert!(state.add_crontab(app_name, crontab_example_1.clone(), retrigger_no));
        assert_eq!(state.ls_crontabs(app_name, None), vec![(
            crontab_example_1.clone(),
            retrigger_no
        )]);
        assert!(state.add_crontab(app_name, crontab_example_2.clone(), retrigger_yes));
        assert_eq!(state.ls_crontabs(app_name, None), vec![
            (crontab_example_1, retrigger_no),
            (crontab_example_2, retrigger_yes),
        ]);
    }
}
