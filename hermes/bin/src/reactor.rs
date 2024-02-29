//! Hermes Reactor implementation.

use std::{sync::Arc, thread};

use crate::{
    app::{HermesApp, IndexedApps},
    event::queue::{event_execution_loop, HermesEventQueue},
    runtime_extensions::hermes::init,
};

/// Thread panics error
#[derive(thiserror::Error, Debug)]
#[error("Thread '{0}' panic! internal error!")]
struct ThreadPanicsError(&'static str);

/// Hermes Reactor struct
#[allow(dead_code)]
pub(crate) struct HermesReactor {
    /// Hermes event queue
    event_queue: Arc<HermesEventQueue>,

    /// Hermes apps
    indexed_apps: IndexedApps,
}

impl HermesReactor {
    /// Create a new Hermes Reactor
    #[allow(dead_code)]
    pub(crate) fn new(apps: Vec<HermesApp>) -> anyhow::Result<Self> {
        let event_queue = Arc::new(HermesEventQueue::new());

        // Emit init event
        init::emit_init_event(
            &event_queue,
            apps.iter().map(|app| app.app_name().clone()).collect(),
        )?;

        let indexed_apps = apps
            .into_iter()
            .map(|app| (app.app_name().clone(), app))
            .collect();

        Ok(Self {
            event_queue,
            indexed_apps,
        })
    }

    /// Run Hermes.
    ///
    /// # Note:
    /// This is a blocking call util all tasks are finished.
    #[allow(dead_code)]
    pub(crate) fn run(self) -> anyhow::Result<()> {
        let events_thread =
            thread::spawn(move || event_execution_loop(&self.indexed_apps, &self.event_queue));

        events_thread
            .join()
            .map_err(|_| ThreadPanicsError("events handler"))??;
        Ok(())
    }
}
