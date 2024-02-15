//! Cron runtime extension implementation.
use std::collections::HashMap;

use crate::runtime_extensions::{
    bindings::hermes::cron::api::{CronEventTag, CronTagged},
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
