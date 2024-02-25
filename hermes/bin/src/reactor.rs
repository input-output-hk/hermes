//! Hermes Reactor implementation.

use std::{sync::Arc, thread};

use crate::{
    app::HermesApp,
    event_queue::HermesEventQueue,
    runtime_extensions::state::{State, Stateful},
};

/// Thread panics error
#[derive(thiserror::Error, Debug)]
#[error("Thread '{0}' panic! internal error!")]
struct ThreadPanicsError(&'static str);

/// Hermes Reactor struct
pub(crate) struct HermesReactor {
    /// Hermes app
    app: HermesApp,

    /// Runtime extensions state
    state: Arc<State>,

    /// Hermes event queue
    event_queue: Arc<HermesEventQueue>,
}

impl HermesReactor {
    /// Create a new Hermes Reactor
    pub(crate) fn new(app_name: &str, module_bytes: Vec<Vec<u8>>) -> anyhow::Result<Self> {
        let app = HermesApp::new(app_name, module_bytes)?;
        let event_queue = HermesEventQueue::new().into();

        let state = State::new().into();

        Ok(Self {
            app,
            state,
            event_queue,
        })
    }

    /// Run Hermes.
    ///
    /// # Note:
    /// This is a blocking call util all tasks are finished.
    pub(crate) fn run(self) -> anyhow::Result<()> {
        // Emits init event
        thread::spawn({
            let event_queue = self.event_queue.clone();
            move || self.state.hermes.init.emit_init_event(event_queue.as_ref())
        });

        let events_thread = thread::spawn(move || self.event_queue.event_execution_loop());

        events_thread
            .join()
            .map_err(|_| ThreadPanicsError("events handler"))??;
        Ok(())
    }
}
