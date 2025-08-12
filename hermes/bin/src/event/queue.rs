//! Hermes event queue implementation.

mod exit;

use std::{
    ops::ControlFlow,
    process::ExitCode,
    sync::mpsc::{self, Receiver, Sender},
    thread::{self},
};

pub use exit::{Exit, ExitLock};
use once_cell::sync::OnceCell;

use super::{HermesEvent, TargetApp, TargetModule};
use crate::{app::ApplicationName, pool, reactor};

/// Singleton instance of the Hermes event queue.
static EVENT_QUEUE_INSTANCE: OnceCell<HermesEventQueue> = OnceCell::new();

/// Failed to add event into the event queue. Event queue is closed.
#[derive(thiserror::Error, Debug, Clone)]
#[error("Failed to add event into the event queue. Event queue is closed.")]
pub(crate) struct CannotAddEventError;

/// Failed to shutdown event queue. Event queue is closed.
#[derive(thiserror::Error, Debug, Clone)]
#[error("Failed to shutdown event queue. Event queue is closed.")]
pub(crate) struct CannotShutdownError;

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
    sender: Sender<ControlFlow<ExitCode, HermesEvent>>,
}

/// Creates a new instance of the `HermesEventQueue`.
/// Runs an event loop thread.
///
/// [`ExitLock`] would contain shutdown information if awaited.
///
/// # Errors:
/// - `AlreadyInitializedError`
pub(crate) fn init() -> anyhow::Result<ExitLock> {
    let (sender, receiver) = std::sync::mpsc::channel();

    EVENT_QUEUE_INSTANCE
        .set(HermesEventQueue { sender })
        .map_err(|_| AlreadyInitializedError)?;

    let (exit_tx, exit_rx) = ExitLock::new_pair();
    thread::spawn(move || {
        let exit = event_execution_loop(&receiver);
        exit_tx.set(exit);
    });
    Ok(exit_rx)
}

/// Add event into the event queue
///
/// # Errors:
/// - `CannotAddEventError`
/// - `NotInitializedError`
pub(crate) fn send(event: HermesEvent) -> anyhow::Result<()> {
    let queue = EVENT_QUEUE_INSTANCE.get().ok_or(NotInitializedError)?;

    queue
        .sender
        .send(ControlFlow::Continue(event))
        .map_err(|_| CannotAddEventError)?;

    Ok(())
}

/// Shutdown the event queue.
///
/// # Errors:
/// - `CannotShutdownQueueError`
/// - `NotInitializedError`
pub(crate) fn shutdown(code: ExitCode) -> anyhow::Result<()> {
    let queue = EVENT_QUEUE_INSTANCE.get().ok_or(NotInitializedError)?;

    queue
        .sender
        .send(ControlFlow::Break(code))
        .map_err(|_| CannotShutdownError)?;

    Ok(())
}

/// Executes provided Hermes event filtering by target module.
fn targeted_module_event_execution(
    target_app_name: &ApplicationName,
    event: &HermesEvent,
) {
    let Ok(app) = reactor::get_app(target_app_name) else {
        tracing::error!("Cannot get app {target_app_name} from reactor");
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
                if let Err(err) = app.dispatch_event_for_target_module(
                    target_module_id.clone(),
                    event.payload().clone(),
                ) {
                    tracing::error!("{err}");
                }
            }
        },
    }
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

/// Executes Hermes events from the provided receiver.
fn event_execution_loop(receiver: &Receiver<ControlFlow<ExitCode, HermesEvent>>) -> Exit {
    loop {
        match receiver.recv() {
            Ok(ControlFlow::Continue(event)) => targeted_app_event_execution(&event),
            Ok(ControlFlow::Break(exit_code)) => {
                if let Err(err) = pool::terminate() {
                    tracing::error!("Failed to terminate thread pool: {err}");
                }
                break Exit::Done { exit_code };
            },
            Err(mpsc::RecvError) => {
                if let Err(err) = pool::terminate() {
                    tracing::error!("Failed to terminate thread pool: {err}");
                }
                break Exit::QueueClosed;
            },
        }
    }
}
