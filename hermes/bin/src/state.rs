//! Hermes state implementation.

use std::sync::Arc;

use crate::{app::HermesAppName, runtime_extensions::state::State, wasm::module::ModuleId};

#[allow(dead_code)]
/// State for Hermes state
pub(crate) struct HermesRuntimeState {
    /// Runtime extensions state
    pub(crate) state: Arc<State>,
    // /// The context of the wasm modules using this State.
    // pub(crate) ctx: Context,
}

impl HermesRuntimeState {
    /// Creates a new instance of the `HermesState`.
    pub(crate) fn new(state: Arc<State>) -> HermesRuntimeState {
        Self { state }
    }
}

/// A Hermes running context, which should be passed to the WASM runtime.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct HermesRuntimeContext {
    /// Hermes application name
    app_name: HermesAppName,

    /// module ULID id
    module_id: ModuleId,

    /// event name to be executed
    event_name: String,

    /// module's execution counter
    exc_counter: u64,
}

impl HermesRuntimeContext {
    /// Creates a new instance of the `Context`.
    pub(crate) fn _new(
        app_name: HermesAppName, module_id: ModuleId, event_name: String, exc_counter: u64,
    ) -> Self {
        Self {
            app_name,
            module_id,
            event_name,
            exc_counter,
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
    pub(crate) fn exc_counter(&self) -> u64 {
        self.exc_counter
    }
}
