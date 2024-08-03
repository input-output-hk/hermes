//! Hermes event queue implementation.

use std::{
    sync::mpsc::{Receiver, Sender},
    thread::{self},
};

use once_cell::sync::OnceCell;

use super::{HermesEvent, TargetApp, TargetModule};
use crate::{app::ApplicationName, reactor};

/// Singleton instance of the Hermes event queue.
static EVENT_QUEUE_INSTANCE: OnceCell<HermesEventQueue> = OnceCell::new();

/// Failed to add event into the event queue. Event queue is closed.
#[derive(thiserror::Error, Debug, Clone)]
#[error("Failed to add event into the event queue. Event queue is closed.")]
pub(crate) struct CannotAddEventError;

/// Failed when event queue already been initialized.
#[derive(thiserror::Error, Debug, Clone)]
#[error("Event queue already been initialized.")]
pub(crate) struct AlreadyInitializedError;

/// Failed when event queue not been initialized.
#[derive(thiserror::Error, Debug, Clone)]
#[error("Event queue not been initialized. Call `init` first.")]
pub(crate) struct NotInitializedError;

/// Hermes event queue.
/// It is a singleton struct.
struct HermesEventQueue {
    /// Hermes event queue sender
    sender: Sender<HermesEvent>,
}

/// Creates a new instance of the `HermesEventQueue`.
/// Runs an event loop thread.
///
/// # Errors:
/// - `AlreadyInitializedError`
pub(crate) fn init() -> anyhow::Result<()> {
    let (sender, receiver) = std::sync::mpsc::channel();

    EVENT_QUEUE_INSTANCE
        .set(HermesEventQueue { sender })
        .map_err(|_| AlreadyInitializedError)?;

    thread::spawn(move || {
        event_execution_loop(receiver);
    });
    Ok(())
}

/// Add event into the event queue
///
/// # Errors:
/// - `CannotAddEventError`
/// - `NotInitializedError`
pub(crate) fn send(event: HermesEvent) -> anyhow::Result<()> {
    let queue = EVENT_QUEUE_INSTANCE.get().ok_or(NotInitializedError)?;

    queue.sender.send(event).map_err(|_| CannotAddEventError)?;

    Ok(())
}

/// Executes provided Hermes event filtering by target module.
fn targeted_module_event_execution(target_app_name: &ApplicationName, event: &HermesEvent) {
    let Ok(Some(app)) = reactor::get_app(target_app_name) else {
        return;
    };

    match event.target_module() {
        TargetModule::All => {
            if let Err(err) = app.dispatch_event(event.payload()) {
                tracing::error!("{err}");
            }
        },
        TargetModule::List(target_modules) => {
            for target_module_id in target_modules {
                if let Err(err) =
                    app.dispatch_event_for_target_module(target_module_id.clone(), event.payload())
                {
                    tracing::error!("{err}");
                }
            }
        },
    };
}

/// Executes provided Hermes event filtering by target app.
fn targeted_app_event_execution(event: &HermesEvent) {
    match event.target_app() {
        TargetApp::All => {
            if let Ok(target_apps) = reactor::get_all_app_names() {
                for target_app_name in target_apps {
                    targeted_module_event_execution(&target_app_name, event);
                }
            }
        },
        TargetApp::List(target_apps) => {
            for target_app_name in target_apps {
                targeted_module_event_execution(target_app_name, event);
            }
        },
    }
}

/// Executes Hermes events from the provided receiver .
fn event_execution_loop(receiver: Receiver<HermesEvent>) {
    for event in receiver {
        targeted_app_event_execution(&event);
    }
}
