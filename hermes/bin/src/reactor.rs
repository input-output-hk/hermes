//! Hermes Reactor implementation.

use std::{sync::Arc, thread};

use crate::{
    app::HermesApp,
    event_queue::{self, HermesEventQueueIn, HermesEventQueueOut},
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

    /// Event queue in
    event_queue_in: HermesEventQueueIn,

    /// Event queue
    event_queue_out: HermesEventQueueOut,
}

impl HermesReactor {
    /// Create a new Hermes Reactor
    pub(crate) fn new(app_name: &str, module_bytes: Vec<Vec<u8>>) -> anyhow::Result<Self> {
        let app = HermesApp::new(app_name, module_bytes)?;
        let (event_queue_in, event_queue_out) = event_queue::new();

        let state = State::new().into();

        Ok(Self {
            app,
            state,
            event_queue_in,
            event_queue_out,
        })
    }

    /// Run Hermes.
    ///
    /// # Note:
    /// This is a blocking call util all tasks are finished.
    pub(crate) fn run(mut self) -> anyhow::Result<()> {
        // Emits init event
        self.state
            .hermes
            .init
            .emit_init_event(&self.event_queue_in)?;

        let events_thread = thread::spawn(move || {
            self.app
                .event_execution_loop(self.event_queue_out, &self.state)
        });

        events_thread
            .join()
            .map_err(|_| ThreadPanicsError("events handler"))??;
        Ok(())
    }
}
