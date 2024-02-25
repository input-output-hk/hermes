//! Hermes event queue implementation.

use std::{
    collections::HashMap,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
};

use super::{HermesEvent, TargetApp, TargetModule};
use crate::{
    app::HermesAppName,
    runtime_extensions::state::State,
    state::HermesState,
    wasm::module::{Module, ModuleId},
};

/// Hermes event queue error
#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum Error {
    /// Target app not found.
    #[error("Target app not found.")]
    AppNotFound,

    /// Target module not found.
    #[error("Target module not found.")]
    ModuleNotFound,

    /// Failed to add event into the event queue. Event queue is closed.
    #[error("Failed to add event into the event queue. Event queue is closed.")]
    CanotAddEvent,

    /// Event execution loop panics in another thread.
    #[error("Event execution loop panics in another thread. error: {0}")]
    ExecutionLoopPanics(String),
}

///
pub(crate) struct HermesEventQueue {
    /// Hermes event queue sender
    sender: Sender<HermesEvent>,
    /// Hermes event queue receiver
    receiver: Mutex<Receiver<HermesEvent>>,

    /// Targets to execute the event
    targets: HashMap<HermesAppName, HashMap<ModuleId, Module>>,
}

impl HermesEventQueue {
    ///
    pub(crate) fn new() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        Self {
            sender,
            receiver: Mutex::new(receiver),

            targets: HashMap::new(),
        }
    }

    /// Add event into the event queue
    pub(crate) fn add_into_queue(&self, event: HermesEvent) -> anyhow::Result<()> {
        self.sender.send(event).map_err(|_| Error::CanotAddEvent)?;
        Ok(())
    }

    /// Executes Hermes events from provided the event queue.
    ///
    /// # Note:
    /// This is a blocking call.
    pub(crate) fn event_execution_loop(&self, state: &Arc<State>) -> anyhow::Result<()> {
        let events = self
            .receiver
            .lock()
            .map_err(|err| Error::ExecutionLoopPanics(err.to_string()))?;

        for event in events.iter() {
            match event.target_app() {
                &TargetApp::All => {
                    for target_modules in self.targets.values_mut() {
                        match event.target_module() {
                            TargetModule::All => {
                                for module in target_modules.values_mut() {
                                    module.execute_event(
                                        event.payload(),
                                        HermesState::new(state.clone()),
                                    )?;
                                }
                            },
                            TargetModule::_List(modules) => {
                                for module_id in modules {
                                    let module = target_modules
                                        .get_mut(module_id)
                                        .ok_or(Error::ModuleNotFound)?;

                                    module.execute_event(
                                        event.payload(),
                                        HermesState::new(state.clone()),
                                    )?;
                                }
                            },
                        }
                    }
                },
                TargetApp::_List(apps) => {
                    for app_name in apps {
                        let target_modules =
                            self.targets.get_mut(app_name).ok_or(Error::AppNotFound)?;

                        match event.target_module() {
                            TargetModule::All => {
                                for module in target_modules.values_mut() {
                                    module.execute_event(
                                        event.payload(),
                                        HermesState::new(state.clone()),
                                    )?;
                                }
                            },
                            TargetModule::_List(modules) => {
                                for module_id in modules {
                                    let module = target_modules
                                        .get_mut(module_id)
                                        .ok_or(Error::ModuleNotFound)?;
                                    module.execute_event(
                                        event.payload(),
                                        HermesState::new(state.clone()),
                                    )?;
                                }
                            },
                        }
                    }
                },
            }
        }
        Ok(())
    }
}
