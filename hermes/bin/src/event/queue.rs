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
    #[error("Target app not found, app name: {0:?}.")]
    AppNotFound(HermesAppName),

    /// Target module not found.
    #[error("Target module not found, module id: {0:?}.")]
    ModuleNotFound(ModuleId),

    /// Failed to add event into the event queue. Event queue is closed.
    #[error("Failed to add event into the event queue. Event queue is closed.")]
    CannotAddEvent,

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
    ///
    /// # Errors:
    /// - `Error::CannotAddEvent`
    pub(crate) fn add_into_queue(&self, event: HermesEvent) -> anyhow::Result<()> {
        self.sender.send(event).map_err(|_| Error::CannotAddEvent)?;
        Ok(())
    }
}

/// Execute a hermes event on the provided module and all necessary info.
///
/// # Errors:
/// - `wasm::module::BadWASMModuleError`
fn execute_event(
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
#[allow(clippy::unnecessary_wraps)]
fn targeted_event_execution(
    indexed_apps: &HashMap<HermesAppName, HashMap<ModuleId, Module>>, event: &HermesEvent,
    state: &Arc<State>,
) -> anyhow::Result<()> {
    match (event.target_app(), event.target_module()) {
        (TargetApp::All, TargetModule::All) => {
            for (app_name, indexed_modules) in indexed_apps {
                for (module_id, module) in indexed_modules {
                    execute_event(
                        app_name.clone(),
                        module_id.clone(),
                        state,
                        event.payload(),
                        module,
                    )?;
                }
            }
        },
        (TargetApp::All, TargetModule::_List(target_modules)) => {
            for (app_name, indexed_modules) in indexed_apps {
                for module_id in target_modules {
                    let module = indexed_modules
                        .get(module_id)
                        .ok_or(Error::ModuleNotFound(module_id.to_owned()))?;

                    execute_event(
                        app_name.clone(),
                        module_id.clone(),
                        state,
                        event.payload(),
                        module,
                    )?;
                }
            }
        },
        (TargetApp::_List(target_apps), TargetModule::All) => {
            for app_name in target_apps {
                let indexed_modules = indexed_apps
                    .get(app_name)
                    .ok_or(Error::AppNotFound(app_name.to_owned()))?;
                for (module_id, module) in indexed_modules {
                    execute_event(
                        app_name.clone(),
                        module_id.clone(),
                        state,
                        event.payload(),
                        module,
                    )?;
                }
            }
        },
        (TargetApp::_List(target_apps), TargetModule::_List(target_modules)) => {
            for app_name in target_apps {
                let indexed_modules = indexed_apps
                    .get(app_name)
                    .ok_or(Error::AppNotFound(app_name.to_owned()))?;
                for module_id in target_modules {
                    let module = indexed_modules
                        .get(module_id)
                        .ok_or(Error::ModuleNotFound(module_id.to_owned()))?;

                    execute_event(
                        app_name.clone(),
                        module_id.clone(),
                        state,
                        event.payload(),
                        module,
                    )?;
                }
            }
        },
    }

    Ok(())
}

/// Executes Hermes events from provided the event queue.
///
/// # Errors:
/// - `Error::AnotherEventExecutionLoop`
/// - `Error::ModuleNotFound`
/// - `Error::AppNotFound`
///
/// # Note:
/// This is a blocking call.
pub(crate) fn event_execution_loop(
    event_queue: &HermesEventQueue, state: &Arc<State>,
) -> anyhow::Result<()> {
    let events = event_queue
        .receiver
        .try_lock()
        .map_err(|_| Error::AnotherEventExecutionLoop)?;

    for event in events.iter() {
        targeted_event_execution(&event_queue.apps, &event, state)?;
    }
    Ok(())
}
