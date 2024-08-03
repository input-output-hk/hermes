//! Hermes Reactor implementation.

use std::sync::Arc;

use dashmap::{mapref::one::Ref, DashMap};
use once_cell::sync::Lazy;

use crate::{
    app::{HermesApp, HermesAppName, IndexedApps},
    event,
    runtime_extensions::hermes::init,
};

/// Global Hermes reactor state
static REACTOR_STATE: Lazy<Reactor> = Lazy::new(|| {
    Reactor {
        apps: DashMap::new(),
    }
});

/// Hermes Reactor struct.
/// This object orchestrates all Hermes apps within all core parts of the Hermes.
struct Reactor {
    /// Loaded hermes apps
    apps: DashMap<HermesAppName, HermesApp>,
}

/// Load Hermes application into the Hermes Reactor.
#[allow(dead_code)]
pub(crate) fn load_app(app: HermesApp) {
    REACTOR_STATE.apps.insert(app.app_name().clone(), app);
}

/// Get Hermes application from the Hermes Reactor.
#[allow(dead_code)]
pub(crate) fn get_app(app_name: &HermesAppName) -> Option<Ref<HermesAppName, HermesApp>> {
    REACTOR_STATE.apps.get(app_name)
}

/// Get all available Hermes application names from the Hermes Reactor.
#[allow(dead_code)]
pub(crate) fn get_all_app_names() -> Vec<HermesAppName> {
    REACTOR_STATE
        .apps
        .iter()
        .map(|val| val.key().clone())
        .collect()
}

/// Hermes Reactor struct
pub(crate) struct HermesReactor {
    /// Hermes apps
    #[allow(dead_code)]
    indexed_apps: Arc<IndexedApps>,
    /// Hermes event queue loop thread handler.
    event_loop: event::queue::HermesEventLoopHandler,
}

impl HermesReactor {
    /// Create a new Hermes Reactor.
    /// Runs all necessary tasks in separate threads.
    pub(crate) fn new(apps: Vec<HermesApp>) -> anyhow::Result<Self> {
        // Loading apps
        let target_apps = apps.iter().map(|app| app.app_name().clone()).collect();

        let indexed_apps: Arc<IndexedApps> = Arc::new(
            apps.into_iter()
                .map(|app| (app.app_name().clone(), app))
                .collect(),
        );

        let event_loop = event::queue::init(indexed_apps.clone())?;

        // Emit Init event for loaded apps
        init::emit_init_event(target_apps)?;

        Ok(Self {
            indexed_apps,
            event_loop,
        })
    }

    /// Waits for all threads to finish.
    /// # Note:
    /// This is a blocking call.
    pub(crate) fn wait(&mut self) -> anyhow::Result<()> {
        self.event_loop.join()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_hermes_reactor_test() {
        let _reactor = HermesReactor::new(vec![]).expect("Could not initialize Hermes reactor.");
    }
}
