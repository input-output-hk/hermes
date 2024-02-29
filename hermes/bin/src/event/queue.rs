//! Hermes event queue implementation.

use std::{
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
    thread,
};

use super::{HermesEvent, HermesEventPayload, TargetApp, TargetModule};
use crate::{
    app::{HermesAppName, IndexedApps},
    runtime_context::HermesRuntimeContext,
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

    /// Panics inside the `event_execution_loop` function error.
    #[error("Panics inside the `event_execution_loop` function!")]
    EventLoopPanics,
}

/// Hermes event queue.
pub(crate) struct HermesEventQueue {
    /// Hermes event queue sender
    sender: Sender<HermesEvent>,
    /// Event loop thread handler
    event_loop: thread::JoinHandle<anyhow::Result<()>>,
}

impl HermesEventQueue {
    /// Creates a new instance of the `HermesEventQueue`.
    /// Runs an event loop thread.
    pub(crate) fn new(indexed_apps: Arc<IndexedApps>) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();

        let event_loop = thread::spawn(move || event_execution_loop(&indexed_apps, receiver));

        Self { sender, event_loop }
    }

    /// Add event into the event queue
    ///
    /// # Errors:
    /// - `Error::CannotAddEvent`
    pub(crate) fn add_into_queue(&self, event: HermesEvent) -> anyhow::Result<()> {
        self.sender.send(event).map_err(|_| Error::CannotAddEvent)?;
        Ok(())
    }

    /// Waits for the event loop to finish.
    /// # Note:
    /// This is a blocking call.
    #[allow(dead_code)]
    pub(crate) fn wait(self) -> anyhow::Result<()> {
        self.event_loop
            .join()
            .map_err(|_| Error::EventLoopPanics)??;
        Ok(())
    }
}

/// Execute a hermes event on the provided module and all necessary info.
///
/// # Errors:
/// - `wasm::module::BadWASMModuleError`
fn execute_event(
    app_name: HermesAppName, module_id: ModuleId, event: &dyn HermesEventPayload, module: &Module,
) -> anyhow::Result<()> {
    let runtime_context = HermesRuntimeContext::new(
        app_name,
        module_id,
        event.event_name().to_string(),
        module.exec_counter(),
    );

    module.execute_event(event, runtime_context)?;
    Ok(())
}

/// Executes provided Hermes event filtering by target app and target module.
///
/// # Errors:
/// - `Error::ModuleNotFound`
/// - `Error::AppNotFound`
/// - `wasm::module::BadWASMModuleError`
#[allow(clippy::unnecessary_wraps)]
fn targeted_event_execution(indexed_apps: &IndexedApps, event: &HermesEvent) -> anyhow::Result<()> {
    match (event.target_app(), event.target_module()) {
        (TargetApp::_All, TargetModule::All) => {
            for (app_name, app) in indexed_apps {
                for (module_id, module) in app.indexed_modules() {
                    execute_event(app_name.clone(), module_id.clone(), event.payload(), module)?;
                }
            }
        },
        (TargetApp::_All, TargetModule::_List(target_modules)) => {
            for (app_name, app) in indexed_apps {
                for module_id in target_modules {
                    let module = app
                        .indexed_modules()
                        .get(module_id)
                        .ok_or(Error::ModuleNotFound(module_id.to_owned()))?;

                    execute_event(app_name.clone(), module_id.clone(), event.payload(), module)?;
                }
            }
        },
        (TargetApp::List(target_apps), TargetModule::All) => {
            for app_name in target_apps {
                let app = indexed_apps
                    .get(app_name)
                    .ok_or(Error::AppNotFound(app_name.to_owned()))?;
                for (module_id, module) in app.indexed_modules() {
                    execute_event(app_name.clone(), module_id.clone(), event.payload(), module)?;
                }
            }
        },
        (TargetApp::List(target_apps), TargetModule::_List(target_modules)) => {
            for app_name in target_apps {
                let app = indexed_apps
                    .get(app_name)
                    .ok_or(Error::AppNotFound(app_name.to_owned()))?;
                for module_id in target_modules {
                    let module = app
                        .indexed_modules()
                        .get(module_id)
                        .ok_or(Error::ModuleNotFound(module_id.to_owned()))?;

                    execute_event(app_name.clone(), module_id.clone(), event.payload(), module)?;
                }
            }
        },
    }

    Ok(())
}

/// Executes Hermes events from the provided receiver .
///
/// # Errors:
/// - `Error::ModuleNotFound`
/// - `Error::AppNotFound`
/// - `wasm::module::BadWASMModuleError`
fn event_execution_loop(
    indexed_apps: &IndexedApps, receiver: Receiver<HermesEvent>,
) -> anyhow::Result<()> {
    for event in receiver {
        targeted_event_execution(indexed_apps, &event)?;
    }
    Ok(())
}
