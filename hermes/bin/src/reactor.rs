//! Hermes Reactor implementation.

use dashmap::{mapref::one::Ref, DashMap};
use once_cell::sync::OnceCell;

use crate::{
    app::{Application, ApplicationName},
    event::{self, queue::ExitLock},
    runtime_extensions::hermes::init,
};

/// Global Hermes reactor state
static REACTOR_STATE: OnceCell<Reactor> = OnceCell::new();

/// Failed when reactor already been initialized.
#[derive(thiserror::Error, Debug, Clone)]
#[error("Reactor already been initialized.")]
pub(crate) struct AlreadyInitializedError;

/// Failed when event queue not been initialized.
#[derive(thiserror::Error, Debug, Clone)]
#[error("Reactor not been initialized. Call `HermesEventQueue::init` first.")]
pub(crate) struct NotInitializedError;

/// Hermes Reactor struct.
/// This object orchestrates all Hermes apps within all core parts of the Hermes.
struct Reactor {
    /// Loaded hermes apps
    apps: DashMap<ApplicationName, Application>,
}

/// Initialize Hermes Reactor.
/// Setup and runs all necessary services.
///
/// [`ExitLock`] would contain shutdown information if awaited.
///
/// # Errors
///
/// - Queue already initialized.
/// - Reactor already initialized.
pub fn init() -> anyhow::Result<ExitLock> {
    let exit_lock = event::queue::init()?;

    REACTOR_STATE
        .set(Reactor {
            apps: DashMap::new(),
        })
        .map_err(|_| AlreadyInitializedError)?;

    Ok(exit_lock)
}

/// Load Hermes application into the Hermes Reactor.
///
/// # Errors
///
/// - Reactor not initialized.
/// - Cannot send initialization event to the application.
pub fn load_app(app: Application) -> anyhow::Result<()> {
    let reactor = REACTOR_STATE.get().ok_or(NotInitializedError)?;

    let app_name = app.name().clone();
    reactor.apps.insert(app_name.clone(), app);

    init::emit_init_event(app_name)?;
    Ok(())
}

/// Get Hermes application from the Hermes Reactor.
pub(crate) fn get_app(
    app_name: &ApplicationName,
) -> anyhow::Result<Ref<ApplicationName, Application>> {
    let reactor = REACTOR_STATE.get().ok_or(NotInitializedError)?;
    reactor
        .apps
        .get(app_name)
        .ok_or(anyhow::anyhow!("Application {app_name} not found"))
}

/// Get all available Hermes application names from the Hermes Reactor.
pub(crate) fn get_all_app_names() -> anyhow::Result<Vec<ApplicationName>> {
    let reactor = REACTOR_STATE.get().ok_or(NotInitializedError)?;
    Ok(reactor.apps.iter().map(|val| val.key().clone()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_test() {
        init().unwrap();
    }
}
