//! Hermes event queue implementation.

use std::{
    collections::HashMap,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
};

use self::event::{HermesEvent, TargetApp, TargetModule};
use crate::{
    app::HermesAppName,
    runtime_extensions::state::State,
    state::HermesState,
    wasm::module::{Module, ModuleId},
};

pub(crate) mod event;

/// Hermes event queue error
#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum Error {
    /// Failed to add event into the event queue. Event queue is closed.
    #[error("Failed to add event into the event queue. Event queue is closed.")]
    QueueClosed,

    /// Target app not found.
    #[error("Target app not found.")]
    AppNotFound,

    /// Target module not found.
    #[error("Target module not found.")]
    ModuleNotFound,
}

///
pub(crate) struct HermesEventExecutionManager {
    /// Targets to execute the event
    targets: HashMap<HermesAppName, HashMap<ModuleId, Module>>,
}

impl HermesEventExecutionManager {
    ///
    pub(crate) fn new() -> Self {
        Self {
            targets: HashMap::new(),
        }
    }

    ///
    fn filtered_execution(
        &mut self, event: &HermesEvent, state: &Arc<State>,
    ) -> anyhow::Result<()> {
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

        Ok(())
    }

    /// Executes Hermes events from provided the event queue.
    ///
    /// # Note:
    /// This is a blocking call and consumes the event queue.
    #[allow(clippy::unnecessary_wraps, clippy::unwrap_used)]
    pub(crate) fn event_execution_loop(
        &mut self, event_queue: &HermesEventQueue, state: &Arc<State>,
    ) -> anyhow::Result<()> {
        for event in event_queue {
            self.filtered_execution(&event, state)?;
        }
        Ok(())
    }
}

/// Hermes event queue
pub(crate) struct HermesEventQueue {
    /// Hermes event queue sender
    sender: Sender<HermesEvent>,
    /// Hermes event queue receiver
    receiver: Mutex<Receiver<HermesEvent>>,
}

impl HermesEventQueue {
    /// Create a new Hermes event queue
    pub(crate) fn new() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();

        Self {
            sender,
            receiver: Mutex::new(receiver),
        }
    }

    /// Add event into the event queue
    pub(crate) fn add(&self, event: HermesEvent) -> anyhow::Result<()> {
        self.sender.send(event).map_err(|_| Error::QueueClosed)?;
        Ok(())
    }
}

impl Iterator for &HermesEventQueue {
    type Item = HermesEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.receiver.lock().unwrap().try_recv().ok()
    }
}
