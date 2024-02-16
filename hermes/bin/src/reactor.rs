//! Hermes Reactor implementation.

use std::{sync::Arc, thread};

use crate::{
    event_queue::{self, HermesEventQueueOut},
    runtime_extensions::state::{State, Stateful},
    wasm::module::Module,
};

/// Thread panics error
#[derive(thiserror::Error, Debug)]
#[error("Thread '{0}' panic! internal error!")]
struct ThreadPanicsError(&'static str);

/// Hermes Reactor struct
pub(crate) struct HermesReactor {
    ///
    wasm_module: Module,

    ///
    state: Arc<State>,

    ///
    event_queue_out: HermesEventQueueOut,
}

impl HermesReactor {
    ///
    fn event_execution_loop(
        wasm_module: &mut Module, event_queue_out: HermesEventQueueOut, state: &Arc<State>,
    ) -> anyhow::Result<()> {
        for event in event_queue_out {
            wasm_module.execute_event(event.as_ref(), state.clone())?;
        }
        Ok(())
    }

    /// Create a new Hermes Reactor
    pub(crate) fn new(app_name: String, module_bytes: &[u8]) -> anyhow::Result<Self> {
        let wasm_module = Module::new(app_name, module_bytes)?;
        let (event_queue_in, event_queue_out) = event_queue::new();

        let state = State::new(&event_queue_in).into();

        Ok(Self {
            wasm_module,
            state,
            event_queue_out,
        })
    }

    ///
    pub(crate) fn run(mut self) -> anyhow::Result<()> {
        // Emits init event
        self.state.hermes.init.emit_init_event()?;

        let events_thread = thread::spawn(move || {
            Self::event_execution_loop(&mut self.wasm_module, self.event_queue_out, &self.state)
        });

        events_thread
            .join()
            .map_err(|_| ThreadPanicsError("events handler"))??;
        Ok(())
    }
}
