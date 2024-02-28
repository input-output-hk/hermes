//! Cron State.
/// The crontabs hash map.
use std::{
    collections::BTreeSet,
    hash::{Hash, Hasher},
    sync::Mutex,
};

use dashmap::DashMap;
//  std::sync::LazyLock is still unstable
use once_cell::sync::Lazy;

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

/// Internal State.
pub(crate) struct InternalState {
    /// The crontabs hash map.
    storage: CronTabStorage,
}

impl InternalState {
    /// Create a new `InternalState`.
    pub(crate) fn new() -> Self {
        Self {
            storage: CronTabStorage::default(),
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
        todo!("implement cron event queue")
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
        todo!("implement cron event queue")
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
