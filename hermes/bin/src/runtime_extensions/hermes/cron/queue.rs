//! Cron Event Queue implementation.

use std::collections::BTreeMap;

use dashmap::DashMap;
use tokio::sync::{mpsc, oneshot};

use super::{
    event::{CronTimestamp, OnCronEvent},
    state::{
        handle_add_cron_job, handle_delay_cron_job, handle_ls_cron_job, handle_rm_cron_job, AppName,
    },
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
    events: DashMap<AppName, BTreeMap<CronTimestamp, Vec<OnCronEvent>>>,
    /// Send events to the crontab queue.
    sender: Option<mpsc::Sender<CronJob>>,
}

impl CronEventQueue {
    /// Create a new `CronQueueTask`.
    pub(crate) fn new(sender: Option<mpsc::Sender<CronJob>>) -> Self {
        Self {
            events: DashMap::default(),
            sender,
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
                        e.push(on_cron_event.clone());
                    })
                    .or_insert_with(|| vec![on_cron_event.clone()]);
            })
            .or_insert_with(|| BTreeMap::from([(timestamp, vec![on_cron_event])]));
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
}

/// The crontab queue task runs in the background.
pub(crate) async fn cron_queue_task(mut queue_rx: mpsc::Receiver<CronJob>) {
    while let Some(cron_job) = queue_rx.recv().await {
        match cron_job {
            CronJob::Add(app_name, on_cron_event, response_tx) => {
                handle_add_cron_job(app_name, on_cron_event, response_tx);
            },
            CronJob::List(app_name, tag, response_tx) => {
                handle_ls_cron_job(&app_name, &tag, response_tx);
            },
            CronJob::Delay(app_name, cron_job_delay, response_tx) => {
                handle_delay_cron_job(app_name, cron_job_delay, response_tx);
            },
            CronJob::Remove(app_name, cron_tagged, response_tx) => {
                handle_rm_cron_job(&app_name, &cron_tagged, response_tx);
            },
        }
    }
}

#[cfg(test)]
mod tests {
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
}
