//! Hermes Reactor implementation.

use std::sync::Arc;

use crate::{
    app::{HermesApp, IndexedApps},
    event,
    runtime_extensions::hermes::init,
};

/// Hermes Reactor struct
#[allow(dead_code)]
pub(crate) struct HermesReactor {
    /// Hermes apps
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
