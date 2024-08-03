//! Hermes Reactor implementation.

use dashmap::{mapref::one::Ref, DashMap};
use once_cell::sync::OnceCell;

use crate::{
    app::{HermesApp, HermesAppName},
    event,
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
    apps: DashMap<HermesAppName, HermesApp>,
}

/// Initialize Hermes Reactor.
/// Setup and runs all necesarry services.
pub(crate) fn init() -> anyhow::Result<()> {
    event::queue::init()?;

    REACTOR_STATE
        .set(Reactor {
            apps: DashMap::new(),
        })
        .map_err(|_| AlreadyInitializedError)?;

    Ok(())
}

/// Load Hermes application into the Hermes Reactor.
pub(crate) fn load_app(app: HermesApp) -> anyhow::Result<()> {
    let reactor = REACTOR_STATE.get().ok_or(NotInitializedError)?;

    let app_name = app.name().clone();
    reactor.apps.insert(app_name.clone(), app);

    init::emit_init_event(app_name)?;
    Ok(())
}

/// Get Hermes application from the Hermes Reactor.
pub(crate) fn get_app(
    app_name: &HermesAppName,
) -> anyhow::Result<Option<Ref<HermesAppName, HermesApp>>> {
    let reactor = REACTOR_STATE.get().ok_or(NotInitializedError)?;
    Ok(reactor.apps.get(app_name))
}

/// Get all available Hermes application names from the Hermes Reactor.
pub(crate) fn get_all_app_names() -> anyhow::Result<Vec<HermesAppName>> {
    let reactor = REACTOR_STATE.get().ok_or(NotInitializedError)?;
    Ok(reactor.apps.iter().map(|val| val.key().clone()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_test() {
        init().expect("Could not initialize Hermes reactor.");
    }
}
