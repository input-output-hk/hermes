//! Hermes event queue implementation.

use std::{
    collections::HashMap,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
};

use super::{HermesEvent, HermesEventPayload, TargetApp, TargetModule};
use crate::{
    app::HermesAppName,
    runtime_extensions::state::State,
    runtime_state::{HermesRuntimeContext, HermesRuntimeState},
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

    /// Trying to execute one more event execution loop. It is allowed to run only one
    /// execution loop in a time.
    #[error("Trying to execute one more event execution loop. It is allowed to run only one execution loop in a time.")]
    AnotherEventExecutionLoop,
}

/// Hermes event queue.
pub(crate) struct HermesEventQueue {
    /// Hermes event queue sender
    sender: Sender<HermesEvent>,
    /// Hermes event queue receiver
    receiver: Mutex<Receiver<HermesEvent>>,

    /// Current available hermes apps with their modules
    apps: HashMap<HermesAppName, HashMap<ModuleId, Module>>,
}

impl HermesEventQueue {
    /// Creates a new instance of the `HermesEventQueue`.
    pub(crate) fn new() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        Self {
            sender,
            receiver: Mutex::new(receiver),

            apps: HashMap::new(),
        }
    }

    /// Add event into the event queue
    pub(crate) fn add_into_queue(&self, event: HermesEvent) -> anyhow::Result<()> {
        self.sender.send(event).map_err(|_| Error::CanotAddEvent)?;
        Ok(())
    }

    /// Execute a hermes event on the provided module and all necesary info.
    fn execute(
        app_name: HermesAppName, module_id: ModuleId, state: &Arc<State>,
        event: &dyn HermesEventPayload, module: &Module,
    ) -> anyhow::Result<()> {
        let runtime_context = HermesRuntimeContext::new(
            app_name,
            module_id,
            event.event_name().to_string(),
            module.exec_counter(),
        );

        let runtime_state = HermesRuntimeState::new(state.clone(), runtime_context);
        module.execute_event(event, runtime_state)?;
        Ok(())
    }

    /// Executes provided Hermes event filtering by target app and target module.
    ///
    /// # Errors:
    /// - `Error::ModuleNotFound`
    /// - `Error::AppNotFound`
    fn filtered_execution(&self, event: &HermesEvent, state: &Arc<State>) -> anyhow::Result<()> {
        let filtered_modules_exec = |target_module: &TargetModule,
                                     app_name: &HermesAppName,
                                     modules: &HashMap<ModuleId, Module>|
         -> anyhow::Result<()> {
            match target_module {
                TargetModule::All => {
                    for (module_id, module) in modules {
                        Self::execute(
                            app_name.clone(),
                            module_id.clone(),
                            state,
                            event.payload(),
                            module,
                        )?;
                    }
                },
                TargetModule::_List(target_modules) => {
                    for module_id in target_modules {
                        let module = modules.get(module_id).ok_or(Error::ModuleNotFound)?;

                        Self::execute(
                            app_name.clone(),
                            module_id.clone(),
                            state,
                            event.payload(),
                            module,
                        )?;
                    }
                },
            }
            Ok(())
        };

        match event.target_app() {
            TargetApp::All => {
                for (app_name, modules) in &self.apps {
                    filtered_modules_exec(event.target_module(), app_name, modules)?;
                }
            },
            TargetApp::_List(apps) => {
                for app_name in apps {
                    let modules = self.apps.get(app_name).ok_or(Error::AppNotFound)?;

                    filtered_modules_exec(event.target_module(), app_name, modules)?;
                }
            },
        }

        Ok(())
    }

    /// Executes Hermes events from provided the event queue.
    ///
    /// # Errors:
    /// - `Error::AnotherEventExecutionLoop` - Trying to execute one more event execution
    ///   loop. It is allowed to run only one execution loop in a time.
    ///
    /// # Note:
    /// This is a blocking call.
    pub(crate) fn event_execution_loop(&self, state: &Arc<State>) -> anyhow::Result<()> {
        let events = self
            .receiver
            .try_lock()
            .map_err(|_| Error::AnotherEventExecutionLoop)?;

        for event in events.iter() {
            self.filtered_execution(&event, state)?;
        }
        Ok(())
    }
}
