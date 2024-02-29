//! Hermes Reactor implementation.

use std::sync::Arc;

use crate::{
    app::{HermesApp, IndexedApps},
    event::queue::HermesEventQueue,
    runtime_extensions::hermes::init,
};

/// Thread panics error
#[derive(thiserror::Error, Debug)]
#[error("Thread '{0}' panic! internal error!")]
struct ThreadPanicsError(&'static str);

/// Hermes Reactor struct
#[allow(dead_code)]
pub(crate) struct HermesReactor {
    /// Hermes event queue
    event_queue: Arc<HermesEventQueue>,

    /// Hermes apps
    indexed_apps: Arc<IndexedApps>,
}

impl HermesReactor {
    /// Create a new Hermes Reactor.
    /// Runs all necessary tasks in separed threads.
    #[allow(dead_code)]
    pub(crate) fn new(apps: Vec<HermesApp>) -> anyhow::Result<Self> {
        let target_apps = apps.iter().map(|app| app.app_name().clone()).collect();

        let indexed_apps: Arc<IndexedApps> = Arc::new(
            apps.into_iter()
                .map(|app| (app.app_name().clone(), app))
                .collect(),
        );

        let event_queue = Arc::new(HermesEventQueue::new(indexed_apps.clone()));

        init::emit_init_event(&event_queue, target_apps)?;

        Ok(Self {
            event_queue,
            indexed_apps,
        })
    }
}
