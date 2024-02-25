//! Hermes Reactor implementation.

use std::{sync::Arc, thread};

use crate::{
    app::HermesApp,
    event_queue::{HermesEventExecutionManager, HermesEventQueue},
    runtime_extensions::state::{State, Stateful},
};

/// Thread panics error
#[derive(thiserror::Error, Debug)]
#[error("Thread '{0}' panic! internal error!")]
struct ThreadPanicsError(&'static str);

/// Hermes Reactor struct
pub(crate) struct HermesReactor {
    /// Runtime extensions state
    state: Arc<State>,

    /// Hermes event queue
    event_queue: Arc<HermesEventQueue>,

    ///
    event_exec_manager: HermesEventExecutionManager,
}

impl HermesReactor {
    /// Create a new Hermes Reactor
    pub(crate) fn new(_apps: Vec<HermesApp>) -> Self {
        let event_queue = HermesEventQueue::new().into();
        let event_exec_manager = HermesEventExecutionManager::new();

        let state = State::new().into();

        Self {
            state,
            event_queue,
            event_exec_manager,
        }
    }

    /// Run Hermes.
    ///
    /// # Note:
    /// This is a blocking call util all tasks are finished.
    pub(crate) fn run(mut self) -> anyhow::Result<()> {
        // Emits init event
        thread::spawn({
            let event_queue = self.event_queue.clone();
            let state = self.state.clone();
            move || state.hermes.init.emit_init_event(event_queue.as_ref())
        });

        let events_thread = thread::spawn({
            let state = self.state.clone();
            let event_queue = self.event_queue.clone();
            move || {
                self.event_exec_manager
                    .event_execution_loop(&event_queue, &state)
            }
        });

        events_thread
            .join()
            .map_err(|_| ThreadPanicsError("events handler"))??;
        Ok(())
    }
}
