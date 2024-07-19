//! Hermes runtime context implementation.

use crate::{app::HermesAppName, vfs::Vfs, wasm::module::ModuleId};

/// Hermes Runtime Context. This is passed to the WASM runtime.
#[derive(Clone, Debug)]
pub(crate) struct HermesRuntimeContext {
    /// Hermes application name
    app_name: HermesAppName,

    /// module's id
    module_id: ModuleId,

    /// event name to be executed
    event_name: String,

    /// module's execution counter
    exc_counter: u32,

    /// App Virtual file system
    #[allow(dead_code)]
    vfs: Option<Vfs>,
}

impl HermesRuntimeContext {
    /// Creates a new instance of the `Context`.
    pub(crate) fn new(
        app_name: HermesAppName, module_id: ModuleId, event_name: String, exc_counter: u32,
        vfs: Option<&Vfs>,
    ) -> Self {
        Self {
            app_name,
            module_id,
            event_name,
            exc_counter,
            vfs: vfs.cloned(),
        }
    }

    /// Get the application name
    #[allow(dead_code)]
    pub(crate) fn app_name(&self) -> &HermesAppName {
        &self.app_name
    }

    /// Get the module id
    #[allow(dead_code)]
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
}
