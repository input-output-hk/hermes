//! Cron Event Queue implementation.

use std::collections::BTreeMap;

use dashmap::DashMap;
use tokio::sync::{mpsc, oneshot};

use super::{
    event::{CronTimestamp, OnCronEvent},
    state::{AppName, CRON_INTERNAL_STATE},
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
    sender: mpsc::Sender<CronJob>,
}
