//! Hermes Reactor implementation.

use std::{sync::Arc, thread};

use crate::{
    app::{HermesApp, IndexedApps},
    event::queue::{event_execution_loop, HermesEventQueue},
    runtime_extensions::state::{State, Stateful},
};

/// Thread panics error
#[derive(thiserror::Error, Debug)]
#[error("Thread '{0}' panic! internal error!")]
struct ThreadPanicsError(&'static str);

/// Hermes Reactor struct
#[allow(dead_code)]
pub(crate) struct HermesReactor {
    /// Runtime extensions state
    state: Arc<State>,

    /// Hermes event queue
    event_queue: Arc<HermesEventQueue>,

    /// Hermes apps
    indexed_apps: IndexedApps,
}

impl HermesReactor {
    /// Create a new Hermes Reactor
    #[allow(dead_code)]
    pub(crate) fn new(apps: Vec<HermesApp>) -> Self {
        let event_queue = HermesEventQueue::new().into();

        let state = State::new().into();
        let indexed_apps = apps
            .into_iter()
            .map(|app| (app.app_name().clone(), app))
            .collect();

        Self {
            state,
            event_queue,
            indexed_apps,
        }
    }

    /// Run Hermes.
    ///
    /// # Note:
    /// This is a blocking call util all tasks are finished.
    #[allow(dead_code)]
    pub(crate) fn run(self) -> anyhow::Result<()> {
        // Emits init event
        self.state
            .hermes
            .init
            .emit_init_event(self.event_queue.as_ref())?;

        let events_thread = thread::spawn({
            let state = self.state.clone();
            move || event_execution_loop(&self.indexed_apps, &self.event_queue, &state)
        });

        events_thread
            .join()
            .map_err(|_| ThreadPanicsError("events handler"))??;
        Ok(())
    }
}
