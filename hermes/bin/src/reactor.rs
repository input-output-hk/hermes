//! Hermes Reactor implementation.

use std::sync::Arc;

use crate::{
    app::{HermesApp, IndexedApps},
    event::queue::{HermesEventLoopHandler, HermesEventQueue},
    runtime_extensions::hermes::init,
};

/// Hermes event queue execution loop handler panics error.
#[derive(thiserror::Error, Debug)]
#[error("Hermes event queue execution loop handler panics!")]
struct EventLoopPanics;

/// Hermes Reactor struct
#[allow(dead_code)]
pub(crate) struct HermesReactor {
    /// Hermes apps
    indexed_apps: Arc<IndexedApps>,
    /// Hermes event queue loop thread handler.
    event_loop: HermesEventLoopHandler,
}

impl HermesReactor {
    /// Create a new Hermes Reactor.
    /// Runs all necessary tasks in separate threads.
    #[allow(dead_code)]
    pub(crate) fn new(apps: Vec<HermesApp>) -> anyhow::Result<Self> {
        // Loading apps
        let target_apps = apps.iter().map(|app| app.app_name().clone()).collect();

        let indexed_apps: Arc<IndexedApps> = Arc::new(
            apps.into_iter()
                .map(|app| (app.app_name().clone(), app))
                .collect(),
        );

        let event_loop = HermesEventQueue::init(indexed_apps.clone())?;

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
    #[allow(dead_code)]
    pub(crate) fn wait(self) -> anyhow::Result<()> {
        self.event_loop.join().map_err(|_| EventLoopPanics)??;
        Ok(())
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
