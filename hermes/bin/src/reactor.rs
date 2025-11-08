//! Hermes Reactor implementation.

use dashmap::{DashMap, mapref::one::Ref};
use once_cell::sync::OnceCell;

use crate::{
    app::{Application, ApplicationName},
    event::{self, queue::ExitLock},
};

/// Global Hermes reactor state
static REACTOR_STATE: OnceCell<Reactor> = OnceCell::new();

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
pub(crate) fn init() -> anyhow::Result<ExitLock> {
    let exit_lock = event::queue::init()?;

    REACTOR_STATE
        .set(Reactor {
            apps: DashMap::new(),
        })
        .map_err(|_| anyhow::anyhow!("Reactor already been initialized."))?;

    Ok(exit_lock)
}

/// Load Hermes application into the Hermes Reactor.
///
/// # Errors
///
/// - Reactor not initialized.
pub(crate) fn load_app(app: Application) -> anyhow::Result<()> {
    let reactor = REACTOR_STATE.get().ok_or(anyhow::anyhow!(
        "Reactor not been initialized. Call `HermesEventQueue::init` first."
    ))?;

    let app_name = app.name().clone();
    reactor.apps.insert(app_name.clone(), app);

    init_app(&app_name)
}

/// Initialize the Application.
pub(crate) fn init_app(app_name: &ApplicationName) -> anyhow::Result<()> {
    let app = get_app(app_name)?;
    app.init()
}

/// Get Hermes application from the Hermes Reactor.
pub(crate) fn get_app(
    app_name: &ApplicationName
) -> anyhow::Result<Ref<'_, ApplicationName, Application>> {
    let reactor = REACTOR_STATE.get().ok_or(anyhow::anyhow!(
        "Reactor not been initialized. Call `HermesEventQueue::init` first."
    ))?;
    reactor
        .apps
        .get(app_name)
        .ok_or(anyhow::anyhow!("Application {app_name} not found"))
}

/// Get all available Hermes application names from the Hermes Reactor.
pub(crate) fn get_all_app_names() -> anyhow::Result<Vec<ApplicationName>> {
    let reactor = REACTOR_STATE.get().ok_or(anyhow::anyhow!(
        "Reactor not been initialized. Call `HermesEventQueue::init` first."
    ))?;
    Ok(reactor.apps.iter().map(|val| val.key().clone()).collect())
}

/// Drops and finalizes all applications in Hermes Reactor.
pub(crate) fn drop_all_apps() -> anyhow::Result<()> {
    let reactor = REACTOR_STATE.get().ok_or(anyhow::anyhow!(
        "Reactor not been initialized. Call `HermesEventQueue::init` first."
    ))?;
    reactor.apps.clear();
    Ok(())
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;

    #[test]
    fn init_test() {
        init().unwrap();
    }
}
