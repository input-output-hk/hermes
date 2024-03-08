//! Cron Event Queue implementation.

use std::collections::BTreeMap;

use dashmap::DashMap;
use tokio::sync::{mpsc, oneshot};

use super::{
    event::{CronTimestamp, OnCronEvent},
    state::{AppName, CRON_INTERNAL_STATE},
    Error,
};
use crate::runtime_extensions::bindings::hermes::cron::api::{CronEventTag, CronTagged};

///
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
    fn add_event(&self, app_name: AppName, timestamp: CronTimestamp, on_cron_event: OnCronEvent) {
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
    fn ls_events(
        &self, app_name: &AppName, cron_tagged: &Option<CronEventTag>,
    ) -> Vec<(CronTagged, bool)> {
        if let Some(app) = self.events.get(app_name) {
            app.iter().fold(vec![], |mut v, (_, cron_events)| {
                //
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
    fn rm_event(&self, app_name: &AppName, cron_tagged: &CronTagged) -> bool {
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
                //
                handle_add_cron_job(app_name, on_cron_event, response_tx);
            },
            CronJob::List(app_name, tag, response_tx) => {
                //
                handle_ls_cron_job(&app_name, &tag, response_tx);
            },
            CronJob::Delay(app_name, cron_job_delay, response_tx) => {
                handle_delay_cron_job(app_name, cron_job_delay, response_tx);
            },
            CronJob::Remove(app_name, cron_tagged, response_tx) => {
                //
                handle_rm_cron_job(&app_name, &cron_tagged, response_tx);
            },
        }
    }
}

/// Handle the `CronJob::Add` command.
fn handle_add_cron_job(
    app_name: AppName, on_cron_event: OnCronEvent, response_tx: oneshot::Sender<bool>,
) {
    let response = if let Some(timestamp) = on_cron_event.next_tick(None) {
        CRON_INTERNAL_STATE
            .cron_queue
            .add_event(app_name, timestamp, on_cron_event);
        true
    } else {
        false
    };
    if let Err(_err) = response_tx.send(response) {
        // TODO(saibatizoku): log error
    }
}

/// Handle the `CronJob::List` command.
fn handle_ls_cron_job(
    app_name: &AppName, cron_tagged: &Option<CronEventTag>,
    response_tx: oneshot::Sender<Vec<(CronTagged, bool)>>,
) {
    let response = CRON_INTERNAL_STATE
        .cron_queue
        .ls_events(app_name, cron_tagged);
    if let Err(_err) = response_tx.send(response) {
        // TODO(saibatizoku): log error
    }
}

/// Handle the `CronJob::Delay` command.
fn handle_delay_cron_job(
    app_name: AppName, CronJobDelay { timestamp, event }: CronJobDelay,
    response_tx: oneshot::Sender<bool>,
) {
    CRON_INTERNAL_STATE
        .cron_queue
        .add_event(app_name, timestamp, event);
    let response = true;
    if let Err(_err) = response_tx.send(response) {
        // TODO(saibatizoku): log error
    }
}

/// Handle the `CronJob::Remove` command.
fn handle_rm_cron_job(
    app_name: &AppName, cron_tagged: &CronTagged, response_tx: oneshot::Sender<bool>,
) {
    let response = CRON_INTERNAL_STATE
        .cron_queue
        .rm_event(app_name, cron_tagged);
    if let Err(_err) = response_tx.send(response) {
        // TODO(saibatizoku): log error
    }
}
