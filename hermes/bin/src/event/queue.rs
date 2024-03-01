//! Hermes event queue implementation.

use std::{
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
};

use once_cell::sync::OnceCell;

use super::{HermesEvent, HermesEventPayload, TargetApp, TargetModule};
use crate::{
    app::{HermesAppName, IndexedApps},
    runtime_context::HermesRuntimeContext,
    wasm::module::{Module, ModuleId},
};

/// Singleton instance of the Hermes event queue.
static EVENT_QUEUE_INSTANCE: OnceCell<HermesEventQueue> = OnceCell::new();

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

    /// Failed when event queue already been initialized.
    #[error("Event queue already been initialized.")]
    AlreadyInitialized,

    /// Failed when event queue not been initialized.
    #[error("Event queue not been initialized. Call `HermesEventQueue::init` first.")]
    NotInitialized,
}

/// Hermes event queue execution loop thread handler
pub(crate) type HermesEventLoopHandler = JoinHandle<anyhow::Result<()>>;

/// Hermes event queue.
/// It is a singleton struct.
pub(crate) struct HermesEventQueue {
    /// Hermes event queue sender
    sender: Sender<HermesEvent>,
}

impl HermesEventQueue {
    /// Creates a new instance of the `HermesEventQueue`.
    /// Runs an event loop thread.
    pub(crate) fn init(indexed_apps: Arc<IndexedApps>) -> anyhow::Result<HermesEventLoopHandler> {
        let (sender, receiver) = std::sync::mpsc::channel();

        EVENT_QUEUE_INSTANCE
            .set(Self { sender })
            .map_err(|_| Error::AlreadyInitialized)?;

        let event_loop = thread::spawn(move || event_execution_loop(&indexed_apps, receiver));

        Ok(event_loop)
    }

    /// Get the singleton instance of the `HermesEventQueue`.
    ///
    /// # Errors:
    /// - `Error::NotInitialized`
    pub(crate) fn get_instance() -> anyhow::Result<&'static HermesEventQueue> {
        Ok(EVENT_QUEUE_INSTANCE.get().ok_or(Error::NotInitialized)?)
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
fn event_dispatch(
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
fn targeted_event_execution(indexed_apps: &IndexedApps, event: &HermesEvent) -> anyhow::Result<()> {
    // Gather target apps
    let target_apps = match event.target_app() {
        TargetApp::_All => indexed_apps.iter().collect(),
        TargetApp::List(target_apps) => {
            let mut res = Vec::new();
            for app_name in target_apps {
                let app = indexed_apps
                    .get(app_name)
                    .ok_or(Error::AppNotFound(app_name.to_owned()))?;
                res.push((app_name, app));
            }
            res
        },
    };
    // Gather target modules
    let target_modules = match event.target_module() {
        TargetModule::All => {
            let mut res = Vec::new();
            for (app_name, app) in target_apps {
                for (module_id, module) in app.indexed_modules() {
                    res.push((app_name, module_id, module));
                }
            }
            res
        },
        TargetModule::_List(target_modules) => {
            let mut res = Vec::new();
            for (app_name, app) in target_apps {
                for module_id in target_modules {
                    let module = app
                        .indexed_modules()
                        .get(module_id)
                        .ok_or(Error::ModuleNotFound(module_id.to_owned()))?;
                    res.push((app_name, module_id, module));
                }
            }
            res
        },
    };

    // Event dispatch
    for (app_name, module_id, module) in target_modules {
        event_dispatch(app_name.clone(), module_id.clone(), event.payload(), module)?;
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
