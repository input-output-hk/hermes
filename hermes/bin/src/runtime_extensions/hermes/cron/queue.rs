//! Cron Event Queue implementation.

use std::collections::{BTreeMap, BTreeSet};

use dashmap::DashMap;
use tokio::sync::{mpsc, oneshot};

use super::{
    event::{CronTimestamp, OnCronEvent},
    state::{cron_queue_delay, cron_queue_trigger, send_hermes_on_cron_event, AppName},
    Error,
};
use crate::runtime_extensions::bindings::hermes::cron::api::{CronEventTag, CronTagged};

/// Cron Job Delay.
#[derive(Debug)]
pub(crate) struct CronJobDelay {
    /// Scheduled time for running the event.
    pub(crate) timestamp: CronTimestamp,
    /// The crontab event.
    pub(crate) event: OnCronEvent,
}

/// Scheduled Date and Time for sending a cron event.
#[derive(Debug)]
pub(crate) enum CronJob {
    /// Add a new cron job for the given app.
    Add(AppName, OnCronEvent, oneshot::Sender<bool>),
    /// List all the cron jobs for the given app.
    List(
        AppName,
        Option<CronEventTag>,
        oneshot::Sender<Vec<(CronTagged, bool)>>,
    ),
    /// Add a delayed cron job for the given app.
    Delay(AppName, CronJobDelay, oneshot::Sender<bool>),
    /// Remove a cron job from the given app.
    Remove(AppName, CronTagged, oneshot::Sender<bool>),
}

/// The crontab queue task runs in the background.
pub(crate) struct CronEventQueue {
    /// The crontab events.
    events: DashMap<AppName, BTreeMap<CronTimestamp, BTreeSet<OnCronEvent>>>,
    /// Send events to the crontab queue.
    sender: Option<mpsc::Sender<CronJob>>,
    /// Next scheduled Cron Task.
    waiting_event: DashMap<usize, (CronTimestamp, std::thread::JoinHandle<()>)>,
}

impl CronEventQueue {
    /// Create a new `CronQueueTask`.
    pub(crate) fn new(sender: Option<mpsc::Sender<CronJob>>) -> Self {
        Self {
            events: DashMap::default(),
            sender,
            waiting_event: DashMap::default(),
        }
    }

    /// Spawn a new cron job.
    pub(crate) fn spawn_cron_job(&self, cron_job: CronJob) -> anyhow::Result<()> {
        Ok(self
            .sender
            .as_ref()
            .ok_or(Error::CronQueueTaskFailed)?
            .blocking_send(cron_job)?)
    }

    /// Add a new crontab entry.
    pub(crate) fn add_event(
        &self, app_name: AppName, timestamp: CronTimestamp, on_cron_event: OnCronEvent,
    ) {
        self.events
            .entry(app_name)
            .and_modify(|e| {
                e.entry(timestamp)
                    .and_modify(|e| {
                        e.insert(on_cron_event.clone());
                    })
                    .or_insert_with(|| BTreeSet::from([on_cron_event.clone()]));
            })
            .or_insert_with(|| BTreeMap::from([(timestamp, BTreeSet::from([on_cron_event]))]));
    }

    /// List all the crontab entries for the given app.
    pub(crate) fn ls_events(
        &self, app_name: &AppName, cron_tagged: &Option<CronEventTag>,
    ) -> Vec<(CronTagged, bool)> {
        if let Some(app) = self.events.get(app_name) {
            app.iter().fold(vec![], |mut v, (_, cron_events)| {
                if let Some(tag) = cron_tagged {
                    for OnCronEvent { tag, last } in cron_events
                        .iter()
                        .filter(|event| event.tag.tag == tag.clone())
                    {
                        v.push((tag.clone(), *last));
                    }
                } else {
                    for OnCronEvent { tag, last } in cron_events {
                        v.push((tag.clone(), *last));
                    }
                };
                v
            })
        } else {
            vec![]
        }
    }

    /// Remove a crontab entry for the given app.
    pub(crate) fn rm_event(&self, app_name: &AppName, cron_tagged: &CronTagged) -> bool {
        let mut response = false;
        if let Some(mut app) = self.events.get_mut(app_name) {
            app.retain(|_ts, events| {
                let start = events.len();
                // Keep `OnCronEvent`s that do not include `cron_tagged`.
                events.retain(|e| e.tag != *cron_tagged);
                let end = events.len();
                // Check if `events` has changed in length, if so, set the `response` to true.
                if start != end {
                    response = true;
                }
                // retain if `events` is not empty
                !events.is_empty()
            });
        }
        response
    }

    /// Trigger the queue.
    ///
    /// This will run until the queue is empty or until the next timestamp in the queue is
    /// in the future, in which case it will update the waiting task, which sleeps until
    /// the next timestamp and calls this function, and return.
    pub(crate) fn trigger(&self) -> anyhow::Result<()> {
        let trigger_time: CronTimestamp = chrono::Utc::now()
            .timestamp_nanos_opt()
            .ok_or(Error::InvalidTimestamp)?
            .try_into()?;
        while let Some((ts, app_names)) = self.next_in_queue() {
            if trigger_time >= ts {
                // If the timestamp is in the past:
                // * send the events immediately
                self.pop_app_queues_and_send(trigger_time, ts, &app_names)?;
            } else {
                // If the timestamp is in the future:
                // * update the waiting task
                self.update_waiting_task(trigger_time, ts);
                // Since `ts` is in the future, we can break
                break;
            }
        }
        Ok(())
    }

    /// Update the waiting task.
    fn update_waiting_task(&self, trigger_time: CronTimestamp, timestamp: CronTimestamp) {
        // Create a new waiting task.
        let duration = timestamp - trigger_time;
        self.waiting_event
            .entry(0)
            .and_modify(|(waiting_for, handle)| {
                // `timestamp` is before the task that is waiting,
                // so we need to update the waiting task, and cancel
                // the old one, if it exists.
                if *waiting_for > timestamp {
                    (*waiting_for, *handle) = new_waiting_task(timestamp, duration);
                }
            })
            .or_insert_with(|| new_waiting_task(timestamp, duration));
    }

    /// Pop the first item from all the `BTreeMap`s belonging
    /// to each `AppName` in the queue. Then send the `OnCronEvent`s
    /// to the Hermes Event Queue.
    ///
    /// This method will also re-schedule the events that have `last = false`.
    fn pop_app_queues_and_send(
        &self, trigger_time: CronTimestamp, ts: CronTimestamp, app_names: &BTreeSet<AppName>,
    ) -> anyhow::Result<()> {
        for app_name in app_names {
            if let Some(events) = self.pop_from_app_queue(app_name, ts) {
                for on_cron_event in events {
                    send_hermes_on_cron_event(app_name.clone(), on_cron_event.clone())?;
                    if !on_cron_event.last {
                        // Re-schedule the event by calculating the next timestamp after now.
                        if let Some(next_timestamp) = on_cron_event.tick_after(None) {
                            let duration = next_timestamp - trigger_time;
                            cron_queue_delay(app_name, duration, on_cron_event.tag.tag)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Pop the first item from the `BTreeMap`.
    ///
    /// Because the `BTreeMap` is sorted, the first item is the smallest timestamp..
    fn pop_from_app_queue(
        &self, app_name: &AppName, timestamp: CronTimestamp,
    ) -> Option<BTreeSet<OnCronEvent>> {
        self.events
            .get_mut(app_name)
            .and_then(|mut app| app.remove(&timestamp))
    }

    /// Get the next timestamp to schedule, collected from all the `BTreeMap`s belonging
    /// to each `AppName` in the queue.
    fn next_in_queue(&self) -> Option<(CronTimestamp, BTreeSet<AppName>)> {
        // Start by fetching the first entry of every app, and putting it into a `BtreeMap`.
        let mut next_events: BTreeMap<CronTimestamp, BTreeSet<AppName>> =
            self.events
                .iter()
                .fold(BTreeMap::new(), |mut acc, mut_ref| {
                    let (app_name, events) = mut_ref.pair();
                    if let Some((ts, _)) = events.first_key_value() {
                        acc.entry(*ts)
                            .and_modify(|e| {
                                e.insert(app_name.clone());
                            })
                            .or_insert_with(|| BTreeSet::from([app_name.clone()]));
                    }
                    acc
                });
        next_events.pop_first()
    }
}

/// Create a new thread that will sleep for `duration` nanoseconds
fn new_waiting_task(
    timestamp: CronTimestamp, duration: u64,
) -> (CronTimestamp, std::thread::JoinHandle<()>) {
    let handle = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_nanos(duration));
        if let Err(_err) = cron_queue_trigger() {
            // TODO (@saibatizoku): log error https://github.com/input-output-hk/hermes/issues/15
        }
    });
    (timestamp, handle)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use super::*;

    const APP_NAME: &str = "test";
    const EVERY_MINUTE_WHEN: &str = "* * * * *";
    const EVERY_DAY_WHEN: &str = "0 0 * * *";
    const EVERY_MONTH_WHEN: &str = "0 0 1 * *";
    const EXAMPLE_TAG: &str = "ExampleTag";
    const OTHER_TAG: &str = "OtherTag";
    const IS_LAST: bool = true;
    const IS_NOT_LAST: bool = false;

    // triggers every minute
    fn cron_entry_1() -> OnCronEvent {
        OnCronEvent {
            tag: CronTagged {
                when: EVERY_MINUTE_WHEN.into(),
                tag: EXAMPLE_TAG.into(),
            },
            last: IS_LAST,
        }
    }
    // triggers every minute
    fn cron_entry_2() -> OnCronEvent {
        OnCronEvent {
            tag: CronTagged {
                when: EVERY_MONTH_WHEN.into(),
                tag: EXAMPLE_TAG.into(),
            },
            last: IS_NOT_LAST,
        }
    }
    // triggers every minute
    fn cron_entry_3() -> OnCronEvent {
        OnCronEvent {
            tag: CronTagged {
                when: EVERY_DAY_WHEN.into(),
                tag: EXAMPLE_TAG.into(),
            },
            last: IS_LAST,
        }
    }
    // triggers every minute
    fn cron_entry_other() -> OnCronEvent {
        OnCronEvent {
            tag: CronTagged {
                when: EVERY_MINUTE_WHEN.into(),
                tag: OTHER_TAG.into(),
            },
            last: IS_LAST,
        }
    }

    #[test]
    fn test_cron_queue_add_and_list_and_remove_events() {
        // Start a queue with no sender channel.
        let queue = CronEventQueue::new(None);

        assert!(queue.ls_events(&APP_NAME.to_string(), &None).is_empty());

        // insert at `CronTimestamp=0`
        queue.add_event(APP_NAME.to_string(), 0, cron_entry_1());
        queue.add_event(APP_NAME.to_string(), 0, cron_entry_2());

        assert_eq!(queue.ls_events(&APP_NAME.to_string(), &None), vec![
            (cron_entry_1().tag, IS_LAST),
            (cron_entry_2().tag, IS_NOT_LAST),
        ]);

        // insert new entry after
        queue.add_event(APP_NAME.to_string(), 180_000_000_000, cron_entry_2());
        // insert other entry after that
        queue.add_event(APP_NAME.to_string(), 360_000_000_000, cron_entry_3());

        assert_eq!(queue.ls_events(&APP_NAME.to_string(), &None), vec![
            (cron_entry_1().tag, IS_LAST),
            (cron_entry_2().tag, IS_NOT_LAST),
            (cron_entry_2().tag, IS_NOT_LAST),
            (cron_entry_3().tag, IS_LAST),
        ]);

        // Insert other entry before the previous two
        queue.add_event(APP_NAME.to_string(), 60_000_000_000, cron_entry_other());

        assert_eq!(queue.ls_events(&APP_NAME.to_string(), &None), vec![
            (cron_entry_1().tag, IS_LAST),
            (cron_entry_2().tag, IS_NOT_LAST),
            (cron_entry_other().tag, IS_LAST),
            (cron_entry_2().tag, IS_NOT_LAST),
            (cron_entry_3().tag, IS_LAST),
        ]);

        // Now remove the events by `CronTagged`
        assert!(queue.rm_event(&APP_NAME.to_string(), &cron_entry_1().tag));
        assert_eq!(queue.ls_events(&APP_NAME.to_string(), &None), vec![
            (cron_entry_2().tag, IS_NOT_LAST),
            (cron_entry_other().tag, IS_LAST),
            (cron_entry_2().tag, IS_NOT_LAST),
            (cron_entry_3().tag, IS_LAST),
        ]);
        assert!(queue.rm_event(&APP_NAME.to_string(), &cron_entry_2().tag));
        assert_eq!(queue.ls_events(&APP_NAME.to_string(), &None), vec![
            (cron_entry_other().tag, IS_LAST),
            (cron_entry_3().tag, IS_LAST),
        ]);
        assert!(queue.rm_event(&APP_NAME.to_string(), &cron_entry_3().tag));
        assert_eq!(queue.ls_events(&APP_NAME.to_string(), &None), vec![(
            cron_entry_other().tag,
            IS_LAST
        ),]);
        assert!(queue.rm_event(&APP_NAME.to_string(), &cron_entry_other().tag));
        // The queue should be empty
        assert!(queue.ls_events(&APP_NAME.to_string(), &None).is_empty());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_cron_queue_pop_from_app_queue() {
        // Start a queue with no sender channel.
        let queue = CronEventQueue::new(None);
        queue.add_event(APP_NAME.to_string(), 0, cron_entry_1());
        queue.add_event(APP_NAME.to_string(), 0, cron_entry_2());
        let events = queue.pop_from_app_queue(&APP_NAME.to_string(), 0).unwrap();
        assert_eq!(events, BTreeSet::from([cron_entry_1(), cron_entry_2()]));
        // The queue should be empty
        assert!(queue.ls_events(&APP_NAME.to_string(), &None).is_empty());

        queue.add_event(APP_NAME.to_string(), 360_000_000_000, cron_entry_3());
        queue.add_event(APP_NAME.to_string(), 180_000_000_000, cron_entry_2());

        let events = queue
            .pop_from_app_queue(&APP_NAME.to_string(), 180_000_000_000)
            .unwrap();
        assert_eq!(events, BTreeSet::from([cron_entry_2()]));

        let events = queue
            .pop_from_app_queue(&APP_NAME.to_string(), 360_000_000_000)
            .unwrap();
        assert_eq!(events, BTreeSet::from([cron_entry_3()]));
        // The queue should be empty
        assert!(queue.ls_events(&APP_NAME.to_string(), &None).is_empty());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_cron_queue_trigger() {
        let queue = CronEventQueue::new(None);

        // To trigger on-cron events, an instance of the `HermesEventQueue` needs to be
        // initialized. Triggering the queue without it, will return error.
        queue.add_event(APP_NAME.to_string(), 0, cron_entry_1());
        assert!(queue.trigger().is_err());

        // Initialize the `HermesEventQueue`
        let _hermes_event_queue = crate::event::queue::init(Arc::new(HashMap::new())).unwrap();
        // Event dispatch is triggered.
        queue.add_event(APP_NAME.to_string(), 0, cron_entry_1());
        // Triggering to `HermesEventQueue` works.
        assert!(queue.trigger().is_ok());
    }
}
