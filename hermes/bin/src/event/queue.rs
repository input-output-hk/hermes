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
    runtime_extensions::new_context,
    wasm::module::{Module, ModuleId},
};

/// Singleton instance of the Hermes event queue.
static EVENT_QUEUE_INSTANCE: OnceCell<HermesEventQueue> = OnceCell::new();

/// Hermes event queue error
#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum Error {
    /// Failed to add event into the event queue. Event queue is closed.
    #[error("Failed to add event into the event queue. Event queue is closed.")]
    CannotAddEvent,

    /// Failed when event queue already been initialized.
    #[error("Event queue already been initialized.")]
    AlreadyInitialized,

    /// Failed when event queue not been initialized.
    #[error("Event queue not been initialized. Call `HermesEventQueue::init` first.")]
    NotInitialized,

    /// Event loop has crashed unexpectedly.
    #[error("Event loop has crashed unexpectedly.")]
    EventLoopPanics,
}

/// Hermes event execution context
type ExecutionContext<'a> = (&'a HermesAppName, &'a ModuleId, &'a Module);

/// Hermes event queue.
/// It is a singleton struct.
struct HermesEventQueue {
    /// Hermes event queue sender
    sender: Sender<HermesEvent>,
}

/// Hermes event queue execution loop thread handler
pub(crate) struct HermesEventLoopHandler {
    /// Hermes event queue execution loop thread handler
    handle: Option<JoinHandle<()>>,
}

impl HermesEventLoopHandler {
    /// Join the event loop thread
    pub(crate) fn join(&mut self) -> anyhow::Result<()> {
        if let Some(handle) = self.handle.take() {
            handle.join().map_err(|_| Error::EventLoopPanics)?;
        }
        Ok(())
    }
}

/// Creates a new instance of the `HermesEventQueue`.
/// Runs an event loop thread.
///
/// # Errors:
/// - `Error::AlreadyInitialized`
pub(crate) fn init(indexed_apps: Arc<IndexedApps>) -> anyhow::Result<HermesEventLoopHandler> {
    let (sender, receiver) = std::sync::mpsc::channel();

    EVENT_QUEUE_INSTANCE
        .set(HermesEventQueue { sender })
        .map_err(|_| Error::AlreadyInitialized)?;

    Ok(HermesEventLoopHandler {
        handle: Some(thread::spawn(move || {
            event_execution_loop(&indexed_apps, receiver);
        })),
    })
}

/// Get execution context
///
/// # Errors:
/// - `Error::AppNotFound`
/// - `Error::ModuleNotFound`
fn get_execution_context<'a>(
    target_app: &'a TargetApp, target_module: &'a TargetModule, indexed_apps: &'a IndexedApps,
) -> Vec<ExecutionContext<'a>> {
    // Gather target apps
    let target_apps = match target_app {
        TargetApp::All => indexed_apps.iter().collect(),
        TargetApp::List(target_apps) => {
            let mut res = Vec::new();
            for app_name in target_apps {
                let Some(app) = indexed_apps.get(app_name) else {
                    tracing::error!("Target app not found, app name: {:?}", app_name);
                    continue;
                };

                res.push((app_name, app));
            }
            res
        },
    };
    // Gather target modules
    match target_module {
        TargetModule::All => {
            let mut res = Vec::new();
            for (app_name, app) in target_apps {
                for (module_id, module) in app.indexed_modules() {
                    res.push((app_name, module_id, module));
                }
            }
            res
        },
        TargetModule::List(target_modules) => {
            let mut res = Vec::new();
            for (app_name, app) in target_apps {
                for module_id in target_modules {
                    let Some(module) = app.indexed_modules().get(module_id) else {
                        tracing::error!(
                            "Target module not found, app name: {:?}, module id: {:?}",
                            app_name,
                            module_id
                        );
                        continue;
                    };

                    res.push((app_name, module_id, module));
                }
            }
            res
        },
    }
}

/// Add event into the event queue
///
/// # Errors:
/// - `Error::AppNotFound`
/// - `Error::ModuleNotFound`
/// - `Error::CannotAddEvent`
/// - `Error::NotInitialized`
pub(crate) fn send(event: HermesEvent) -> anyhow::Result<()> {
    let queue = EVENT_QUEUE_INSTANCE.get().ok_or(Error::NotInitialized)?;

    queue
        .sender
        .send(event)
        .map_err(|_| Error::CannotAddEvent)?;

    Ok(())
}

/// Execute a hermes event on the provided module and all necessary info.
///
/// # Errors:
/// - `wasm::module::BadWASMModuleError`
pub(crate) fn event_dispatch(
    app_name: HermesAppName, module_id: ModuleId, module: &Module, event: &dyn HermesEventPayload,
) {
    let runtime_context = HermesRuntimeContext::new(
        app_name,
        module_id,
        event.event_name().to_string(),
        module.exec_counter(),
    );

    // Advise Runtime Extensions of a new context
    new_context(&runtime_context);

    if let Err(err) = module.execute_event(event, runtime_context) {
        tracing::error!("Error executing event, err: {err}");
    }
}

/// Executes provided Hermes event filtering by target app and target module.
fn targeted_event_execution(indexed_apps: &IndexedApps, event: &HermesEvent) {
    let execution_contexts =
        get_execution_context(event.target_app(), event.target_module(), indexed_apps);

    // Event dispatch
    for (app_name, module_id, module) in execution_contexts {
        event_dispatch(app_name.clone(), module_id.clone(), module, event.payload());
    }
}

/// Executes Hermes events from the provided receiver .
///
/// # Errors:
/// - `wasm::module::BadWASMModuleError`
fn event_execution_loop(indexed_apps: &IndexedApps, receiver: Receiver<HermesEvent>) {
    for event in receiver {
        targeted_event_execution(indexed_apps, &event);
    }
}
