//! Hermes runtime context implementation.

use std::sync::Arc;

use crate::{app::ApplicationName, vfs::Vfs, wasm::module::ModuleId};

/// Hermes Runtime Context. This is passed to the WASM runtime.
#[derive(Clone, Debug)]
pub(crate) struct HermesRuntimeContext {
    /// Hermes application name
    app_name: ApplicationName,

    /// module's id
    module_id: ModuleId,

    /// event name to be executed
    event_name: String,

    /// module's execution counter
    exc_counter: u32,

    /// App Virtual file system
    vfs: Arc<Vfs>,
}

impl HermesRuntimeContext {
    /// Creates a new instance of the `Context`.
    pub(crate) fn new(
        app_name: ApplicationName, module_id: ModuleId, event_name: String, exc_counter: u32,
        vfs: Arc<Vfs>,
    ) -> Self {
        Self {
            app_name,
            module_id,
            event_name,
            exc_counter,
            vfs,
        }
    }

    /// Get the application name
    pub(crate) fn app_name(&self) -> &ApplicationName {
        &self.app_name
    }

    /// Get the module id
    pub(crate) fn module_id(&self) -> &ModuleId {
        &self.module_id
    }

    /// Get the event name
    #[allow(dead_code)]
    pub(crate) fn event_name(&self) -> &str {
        self.event_name.as_ref()
    }

    /// Get the counter value
    #[allow(dead_code)]
    pub(crate) fn exc_counter(&self) -> u32 {
        self.exc_counter
    }

    /// Get virtual file system
    #[allow(dead_code)]
    pub(crate) fn vfs(&self) -> &Vfs {
        self.vfs.as_ref()
    }
}
